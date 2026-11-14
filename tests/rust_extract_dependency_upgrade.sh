#!/usr/bin/env bash
# CI-only resolver regression: recreate the relevant old-lock graph, then verify
# the new manifest reconciled it. Never run against the user's existing lock.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
manifest="$root/prototypes/rust-extract/Cargo.toml"
lock="$root/prototypes/rust-extract/Cargo.lock"
case "${1:-}" in
  seed)
    if [[ -e "$lock" ]]; then
      echo "Refusing to overwrite an existing prototype lock; use a clean CI checkout." >&2
      exit 2
    fi
    scratch=$(mktemp -d)
    trap 'rm -rf -- "$scratch"' EXIT
    mkdir "$scratch/src"
    printf 'fn main() {}\n' > "$scratch/src/main.rs"
    cat > "$scratch/Cargo.toml" <<'EOF'
[package]
name = "aysha-extract-prototype"
version = "0.1.0"
edition = "2021"
[dependencies]
url = "=2.5.8"
idna_adapter = "=1.2.2"
EOF
    cargo generate-lockfile --manifest-path "$scratch/Cargo.toml"
    python3 - "$scratch/Cargo.lock" <<'PY'
import sys, tomllib
with open(sys.argv[1], 'rb') as file:
    packages = tomllib.load(file)['package']
assert any(p['name'] == 'idna_adapter' and p['version'] == '1.2.2' for p in packages)
assert any(p['name'] == 'icu_normalizer' and tuple(map(int, p['version'].split('.'))) >= (2, 2, 0) for p in packages)
PY
    cp -- "$scratch/Cargo.lock" "$lock"
    echo "Seeded clean CI checkout with the host's conflicting adapter/ICU constraint shape."
    ;;
  verify)
    python3 - "$lock" <<'PY'
import sys, tomllib
with open(sys.argv[1], 'rb') as file:
    packages = tomllib.load(file)['package']
versions = {name: sorted({p['version'] for p in packages if p['name'] == name})
            for name in ('idna_adapter', 'boa_engine', 'icu_normalizer')}
print('::notice title=Resolved prototype dependency versions::' + str(versions))
checks = [
    (versions['idna_adapter'] == ['1.2.1'], 'adapter pin not applied'),
    (bool(versions['boa_engine']) and all(v.startswith('0.21.') for v in versions['boa_engine']), 'unexpected Boa family'),
    (bool(versions['icu_normalizer']) and all(v.startswith('2.0.') for v in versions['icu_normalizer']), 'unexpected ICU normalizer'),
]
for passed, reason in checks:
    if not passed:
        print('::error title=Prototype lock migration check::' + reason + ': ' + str(versions))
        raise SystemExit(1)
print('PASS: existing-lock upgrade resolves IDNA 1.2.1 / Boa 0.21.x / ICU normalizer 2.0.x')
PY
    # --locked proves verification itself does not require another resolution change.
    cargo metadata --locked --format-version 1 --manifest-path "$manifest" > /dev/null
    ;;
  *) echo "CI-only usage: $0 seed|verify" >&2; exit 2 ;;
esac
