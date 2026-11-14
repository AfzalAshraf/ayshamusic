#!/usr/bin/env python3
"""Read-only running-server identity and RAM timeline. No playback controls or builds.

Run during ordinary user-controlled listening, never alongside a build. This is
stdlib diagnostic tooling, not a production Python worker or a background daemon.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import tempfile
import time

try:
    from scripts import check_health as health, check_playback_budget as budget
except ModuleNotFoundError:
    import check_health as health
    import check_playback_budget as budget

LABELS = {"music-server", "yt-dlp", "python3", "node", "mpv", "ffmpeg"}


def text(path, limit=65536):
    with path.open('rb') as source:
        value = source.read(limit + 1)
    if len(value) > limit:
        raise ValueError('bounded read exceeded')
    return value.decode('utf8')


def digest(path):
    hasher = hashlib.sha256()
    with path.open('rb') as source:
        if os.fstat(source.fileno()).st_size > 64 * 1024 * 1024:
            raise ValueError('image exceeds diagnostic limit')
        remaining = 64 * 1024 * 1024
        while remaining:
            block = source.read(min(65536, remaining))
            if not block:
                return hasher.hexdigest()
            hasher.update(block)
            remaining -= len(block)
        if source.read(1):
            raise ValueError('image exceeds diagnostic limit')
    return hasher.hexdigest()


def start_ticks(process):
    try:
        fields = text(process / 'stat', 16384).rsplit(')', 1)[1].split()
        value = fields[19]
        return value if value.isascii() and value.isdigit() else None
    except (OSError, ValueError, IndexError):
        return None


def listens_on_5050(process):
    inodes = set()
    readable = False
    complete = True
    for name in ['tcp', 'tcp6']:
        try:
            for line in text(process / 'net' / name, 256 * 1024).splitlines()[1:]:
                parts = line.split()
                if len(parts) >= 10 and parts[3] == '0A' and parts[1].rsplit(':', 1)[-1].upper() == '13BA':
                    inodes.add(parts[9])
            readable = True
        except FileNotFoundError:
            continue
        except (OSError, ValueError):
            complete = False
    try:
        with os.scandir(process / 'fd') as entries:
            for index, entry in enumerate(entries):
                if index >= 256:
                    complete = False
                    break
                try:
                    target = os.readlink(entry.path)
                    if target.startswith('socket:[') and target.endswith(']') and target[8:-1] in inodes:
                        return True
                except OSError:
                    complete = False
    except OSError:
        complete = False
    return False if readable and complete else None


def running_image(pid, proc_root=Path('/proc'), installed=health.INSTALLED):
    result = {'service': health.SERVICE, 'main_process': 'unverified',
              'loaded_sha256': None, 'installed_sha256': None,
              'loaded_matches_installed': None, 'main_process_listens_on_5050': None,
              'image_identity_stable_during_read': None,
              'limits': 'Loaded image vs installed file, not proof of source revision, clean build or authentication. No PIDs, paths, argv or environment returned.'}
    if not isinstance(pid, str) or not re.fullmatch(r'[0-9]{1,12}', pid) or int(pid) == 0:
        return result, None
    process = proc_root / pid
    token = start_ticks(process)
    try:
        comm = text(process / 'comm', 256).strip()
        result['main_process'] = 'music-server' if comm == 'music-server' else 'unexpected_process'
        result['main_process_listens_on_5050'] = listens_on_5050(process)
        result['loaded_sha256'] = digest(process / 'exe')
        result['installed_sha256'] = digest(installed)
        result['loaded_matches_installed'] = result['loaded_sha256'] == result['installed_sha256']
    except (OSError, ValueError):
        pass
    if token is not None:
        result['image_identity_stable_during_read'] = token == start_ticks(process)
        if not result['image_identity_stable_during_read']:
            result['loaded_matches_installed'] = None
    return result, token


def runtime_status():
    started = time.monotonic()
    result = {'available': False, 'transport': 'unverified', 'playing_flag': None, 'rust_stream_active': None}
    try:
        value = json.loads(health.http_get('/api/status'))
        if not isinstance(value, dict):
            raise ValueError('invalid schema')
        mode = value.get('stream_transport')
        if isinstance(mode, str) and mode in {'legacy', 'rust-resolved', 'invalid'}:
            result['transport'] = mode
        for key, source in [('playing_flag', 'is_playing'), ('rust_stream_active', 'rust_stream_active')]:
            flag = value.get(source)
            result[key] = flag if type(flag) is bool else None
        result['available'] = type(value.get('is_playing')) is bool
    except (OSError, ValueError):
        pass
    result['status_request_ms'] = round((time.monotonic() - started) * 1000, 2)
    return result


def host_memory(proc_root=Path('/proc')):
    try:
        fields = {}
        for line in text(proc_root / 'meminfo').splitlines():
            parts = line.split()
            if len(parts) == 3 and parts[0] in {'MemTotal:', 'MemAvailable:', 'SwapTotal:', 'SwapFree:'} and parts[2] == 'kB' and parts[1].isascii() and parts[1].isdigit():
                fields[parts[0][:-1]] = int(parts[1]) * 1024
        total, available = fields['MemTotal'], fields['MemAvailable']
        if not 0 <= available <= total:
            raise ValueError('invalid counters')
        swap_total, swap_free = fields['SwapTotal'], fields['SwapFree']
        if not 0 <= swap_free <= swap_total:
            raise ValueError('invalid swap counters')
        return {'total_bytes': total, 'available_bytes': available,
                'nonavailable_estimate_bytes': total - available, 'swap_used_bytes': swap_total - swap_free}
    except (OSError, ValueError, KeyError):
        return None


def safe_buckets(buckets):
    result = {}
    for label, (count, rss) in buckets.items():
        label = label if label in LABELS else 'other'
        old = result.setdefault(label, {'count': 0, 'rss_bytes': 0})
        old['count'] += count
        old['rss_bytes'] += rss
    return result


def progress(row):
    current = row.get('memory', {}).get('memory.current')
    size = 'unavailable' if current is None else f'{current / 1_000_000:.2f} MB'
    buckets = row['process_rss_buckets']
    print(f"t={row['elapsed_seconds']:.1f}s service={size} "
          f"yt-dlp={buckets.get('yt-dlp', {}).get('count', 0)} node={buckets.get('node', {}).get('count', 0)} "
          f"playing={row['runtime']['playing_flag']} transfer={row['runtime']['rust_stream_active']}", flush=True)


def collect(count=30, interval=2, root=Path('/sys/fs/cgroup'), proc_root=Path('/proc'), notify=None):
    before = budget.service_state()
    group = budget.group_path(before.get('ControlGroup', ''), root)
    image, token = running_image(before['MainPID'], proc_root)
    started = time.monotonic()
    start_utc = datetime.now(timezone.utc).isoformat()
    timeline = []
    incomplete = False
    interrupted = False
    try:
        for index in range(count):
            if time.monotonic() - started >= 180:
                break
            row = {'index': index + 1, 'utc': datetime.now(timezone.utc).isoformat(),
                   'elapsed_seconds': round(time.monotonic() - started, 3), 'process_rss_buckets': {}}
            try:
                values, buckets, partial = health.cgroup_snapshot(group, proc_root)
                row.update(memory=values, process_rss_buckets=safe_buckets(buckets), inventory_incomplete=partial)
                incomplete |= partial
            except (OSError, ValueError):
                row['inventory_incomplete'] = True
                incomplete = True
            row['runtime'] = runtime_status()
            row['host_all_workloads'] = host_memory(proc_root)
            timeline.append(row)
            if notify is not None:
                notify(row)
            if index + 1 < count:
                delay = min(max(0, started + (index + 1) * interval - time.monotonic()),
                            max(0, 180 - (time.monotonic() - started)))
                if delay:
                    time.sleep(delay)
    except KeyboardInterrupt:
        interrupted = True
    try:
        after = budget.service_state()
        stable = all(before.get(key) == after.get(key) for key in ['ControlGroup', 'MainPID', 'NRestarts'])
        if token is not None:
            stable &= token == start_ticks(proc_root / before['MainPID'])
    except (OSError, ValueError):
        stable = False
    memory_rows = [row for row in timeline if 'memory' in row]
    flags = [timeline[0]['runtime']['playing_flag'], timeline[-1]['runtime']['playing_flag']] if timeline else [None, None]
    report = budget.evaluate([row['memory'] for row in memory_rows], flags, stable, incomplete)
    peak_row = max(memory_rows, key=lambda row: row['memory']['memory.current']) if memory_rows else None
    maxima = {}
    for row in timeline:
        for label, values in row['process_rss_buckets'].items():
            old = maxima.setdefault(label, {'max_count': 0, 'max_rss_bytes': 0})
            old['max_count'] = max(old['max_count'], values['count'])
            old['max_rss_bytes'] = max(old['max_rss_bytes'], values['rss_bytes'])
    latency = [row['runtime']['status_request_ms'] for row in timeline if row['runtime']['available']]
    identity_status = ('fail' if image['main_process'] == 'unexpected_process' or image['main_process_listens_on_5050'] is False
                       else 'observed' if image['main_process_listens_on_5050'] is True and image['image_identity_stable_during_read'] is True and stable else 'unverified')
    latest = timeline[-1] if timeline else None
    memory = latest.get('memory', {}) if latest else {}
    host = latest['host_all_workloads'] if latest else None
    ram_output = {'observed_utc': latest['utc'] if latest else None,
                  'configured_transport': latest['runtime']['transport'] if latest else 'unverified',
                  'service_current_MB': memory.get('memory.current', 0) / 1_000_000 if 'memory.current' in memory else None,
                  'service_historical_peak_MB': memory.get('memory.peak', 0) / 1_000_000 if 'memory.peak' in memory else None,
                  'service_swap_MB': memory.get('memory.swap.current', 0) / 1_000_000 if 'memory.swap.current' in memory else None,
                  'window_max_service_MB': report.get('sampled_max_current_MB'),
                  'host_nonavailable_estimate_MB': host['nonavailable_estimate_bytes'] / 1_000_000 if host else None,
                  'host_physical_total_MB': host['total_bytes'] / 1_000_000 if host else None,
                  'whole_application_total_MB': None,
                  'whole_application_total_reason': 'Shared platform/escaped worker attribution not established; host total is not music-only.'}
    report.update(schema=1, candidate='aysha-runtime-memory-monitor-v1', running_server=image,
                  runtime_identity_status=identity_status, source_revision_certified=False, ram_output=ram_output,
                  window_start_utc=start_utc, window_end_utc=datetime.now(timezone.utc).isoformat(),
                  samples_requested=count, samples_collected=len(timeline), interval_seconds=interval,
                  window_complete=(len(memory_rows) == count and stable and not interrupted), interrupted=interrupted,
                  latest=timeline[-1] if timeline else None, maximum_current_sample=peak_row,
                  process_rss_bucket_maxima=maxima,
                  status_request_median_ms=statistics.median(latency) if latency else None,
                  timeline=timeline, starts_playback=False, upstream_requested=False, builds_run=False,
                  additional_limits='Status GET timing is NOT Play/Next/Previous latency. Host counters include ALL other workloads/OS, not music-only. Service cgroup includes descendants, NOT shared audio/BlueZ/D-Bus/tunnel or escaped workers. Never add host/service/RSS numbers. A false transfer flag can mean buffered playback. Hash equality does not certify source revision. Sampling can miss peaks; no counter resets. The monitor is not a daemon or remotely installed agent.')
    return report


def save(report):
    directory = health.ROOT / 'diagnostics'
    directory.mkdir(mode=0o700, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix='server-memory-', suffix='.json', dir=directory)
    with os.fdopen(fd, 'w') as target:
        json.dump(report, target, indent=2)
    return 'diagnostics/' + Path(name).name


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=int, default=30, help='1–60 samples, no more than about 180s of sampling')
    parser.add_argument('--interval', type=int, choices=[1, 2, 5], default=2)
    args = parser.parse_args(argv)
    if not 1 <= args.samples <= 60:
        parser.error('samples must be 1–60')
    try:
        report = collect(args.samples, args.interval, notify=progress)
        path = save(report)
    except (OSError, ValueError):
        print(json.dumps({'status': 'incomplete', 'code': 'monitor_or_private_save_unavailable',
                          'whole_stack_accepted': False, 'service_changed': False}))
        return 3
    summary = {key: value for key, value in report.items() if key != 'timeline'}
    summary['saved_report'] = path
    print('=== BEGIN SERVER RAM SUMMARY ===')
    print(json.dumps(summary, indent=2))
    print('=== END SERVER RAM SUMMARY ===')
    return 1 if report['status'] == 'fail' or report['runtime_identity_status'] == 'fail' else 2


if __name__ == '__main__':
    raise SystemExit(main())
