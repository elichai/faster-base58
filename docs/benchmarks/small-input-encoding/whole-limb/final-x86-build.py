"""Rebuild and retime final typed-chunk source at the same canonical x86 paths."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile

root = Path('/dev/shm/base58-pr1-canonical')
source = root / 'source'
previous = json.loads((root / 'candidate/metadata.json').read_text())
snapshot = root / 'final-source'
snapshot.mkdir(exist_ok=False)
with tarfile.open('/dev/shm/base58-pr1-final-source.tar') as archive:
    assert set(archive.getnames()) == {'src/encode/mod.rs', 'src/encode/matrix.rs', 'src/encode/tests.rs'}
    archive.extractall(snapshot)
for path in snapshot.rglob('*.rs'):
    relative = str(path.relative_to(snapshot))
    destination = source / relative
    assert hashlib.sha256(destination.read_bytes()).hexdigest() == previous['source_files_sha256'][relative]
    destination.write_bytes(path.read_bytes())
env = dict(os.environ, PATH=str(Path.home() / '.cargo/bin') + ':' + os.environ['PATH'], TMPDIR=str(root / 'tmp'))
subprocess.run(['python3', str(source / 'scripts/code_size.py'), '--source', str(source),
                '--output', str(root / 'build'), '--target', 'x86_64-unknown-linux-gnu',
                '--backends', 'auto,no-alloc', '--directions', 'both', '--kinds', 'cdylib',
                '--cpu', 'native', '--strip-tool', shutil.which('strip')], env=env, check=True)
(root / 'build').rename(root / 'final')
os.sched_setaffinity(0, {min(os.sched_getaffinity(0))})
for backend in ('auto', 'no-alloc'):
    output = root / ('final-native-' + backend + '.json')
    subprocess.run([sys.executable, str(source / 'docs/benchmarks/small-input-encoding/compare.py'),
                    str(root / 'reference' / (backend + '-both-cdylib.so')),
                    str(root / 'final' / (backend + '-both-cdylib.so')), str(output),
                    '16', '22', '31', '32', '33', '64', '65', '128', '512'], check=True)
