"""Exercise packaged yt-dlp downloading a synthetic audio stream from loopback.
No YouTube/provider request is made. Temporary files stay in the runner temp dir.
"""
import functools
import http.server
import json
import pathlib
import subprocess
import sys
import tempfile
import threading

root = pathlib.Path(__file__).resolve().parent
binary = pathlib.Path(sys.argv[1]).resolve()
selector = 'bestaudio[acodec^=opus][ext=webm]/bestaudio[acodec^=mp4a][ext=m4a]/bestaudio[acodec^=mp4a][ext=mp4]'

class Silent(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass

server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(Silent, directory=str(root)))
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()
try:
    for filename, ext, codec in [('tone-opus.webm', 'webm', 'opus'), ('tone.m4a', 'm4a', 'mp4a.40.2')]:
        data = {'id': 'fixture0001', 'title': 'offline tone', 'extractor': 'generic',
                'formats': [{'format_id': 'audio', 'url': f'http://127.0.0.1:{server.server_port}/{filename}',
                             'ext': ext, 'vcodec': 'none', 'acodec': codec, 'abr': 128}]}
        with tempfile.TemporaryDirectory() as directory:
            info = pathlib.Path(directory) / 'fixture.json'
            info.write_text(json.dumps(data), encoding='utf-8')
            result = subprocess.run([str(binary), '--ignore-config', '--no-plugin-dirs', '--no-cache-dir',
                '--no-playlist', '--no-progress', '--proxy', '', '--no-remote-components', '--no-js-runtimes',
                '--no-check-formats', '--retries', '0', '--fragment-retries', '0', '--extractor-retries', '0',
                '--socket-timeout', '15', '--max-filesize', '64M', '--load-info-json', str(info),
                '-f', selector, '-o', '-'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
            if result.returncode or result.stdout != (root / filename).read_bytes():
                raise RuntimeError(f'offline download fixture failed for {ext}: {result.stderr.decode(errors="replace")}')
            print(f'offline {ext} download: bytes match')
finally:
    server.shutdown()
    server.server_close()
    thread.join()
