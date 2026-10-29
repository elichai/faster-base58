"""Reuse PR1's original physical probe paths while preserving immutable originals."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

probe = Path('/private/tmp/base58-pr1-size-pr')
source = Path('/private/tmp/base58-pr1-review')
destination = Path('/private/tmp/base58-pr1-canonical-arm')
destination.mkdir(exist_ok=False)
metadata = json.loads((probe / 'metadata.json').read_text())
env = dict(os.environ, CARGO_TARGET_DIR=str(probe / 'target'), RUSTFLAGS=metadata['rustflags'])
env.pop('CARGO_ENCODED_RUSTFLAGS', None)
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
changed = ('src/encode/mod.rs', 'src/encode/matrix.rs', 'src/encode/tests.rs')
source_hashes = {name: digest(source / name) for name in changed}
original_hashes = {backend: digest(probe / (backend + '-both-cdylib.dylib')) for backend in ('auto', 'no-alloc')}
for backend, command in zip(('auto', 'no-alloc'), metadata['commands']):
    subprocess.run(command, env=env, check=True)
    shutil.copyfile(probe / 'target/aarch64-apple-darwin/release/libbase58_code_size.dylib',
                    destination / (backend + '-both-cdylib.dylib'))
assert source_hashes == {name: digest(source / name) for name in changed}
assert original_hashes == {backend: digest(probe / (backend + '-both-cdylib.dylib')) for backend in original_hashes}
(destination / 'identity.json').write_text(json.dumps({'source_hashes': source_hashes, 'original_artifact_hashes': original_hashes,
    'candidate_artifact_hashes': {backend: digest(destination / (backend + '-both-cdylib.dylib')) for backend in original_hashes},
    'probe_manifest': str(probe / 'probe/Cargo.toml'), 'commands': metadata['commands'], 'rustflags': metadata['rustflags']}, indent=2) + '\n')
for backend in ('auto', 'no-alloc'):
    subprocess.run([sys.executable, str(source / 'docs/benchmarks/small-input-encoding/compare.py'),
                    str(probe / (backend + '-both-cdylib.dylib')), str(destination / (backend + '-both-cdylib.dylib')),
                    str(destination / (backend + '.json')), '16', '22', '31', '32', '33', '63', '64', '65', '128', '512'], check=True)
