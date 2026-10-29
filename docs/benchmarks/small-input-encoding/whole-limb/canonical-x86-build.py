"""Remove source/probe-path differences from the native PR1 comparison."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys

root = Path('/dev/shm/base58-pr1-canonical')
fixtures = Path('/dev/shm/base58-pr1-improve')
reference = fixtures / 'base58-pr1-reference'
candidate = fixtures / 'base58-pr1-ten-top'
root.mkdir(exist_ok=False)
source = root / 'source'
shutil.copytree(reference, source)
(root / 'tmp').mkdir()
env = dict(os.environ, PATH=str(Path.home() / '.cargo/bin') + ':' + os.environ['PATH'], TMPDIR=str(root / 'tmp'))
changes = ('src/encode/mod.rs', 'src/encode/matrix.rs', 'src/encode/tests.rs')
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

for variant in ('reference', 'candidate'):
    if variant == 'candidate':
        for relative in changes:
            destination = source / relative
            assert digest(destination) == digest(reference / relative), relative
            destination.write_bytes((candidate / relative).read_bytes())
            assert digest(destination) == digest(candidate / relative), relative
    output = root / 'build'
    subprocess.run(['python3', str(source / 'scripts/code_size.py'), '--source', str(source),
                    '--output', str(output), '--target', 'x86_64-unknown-linux-gnu',
                    '--backends', 'auto,no-alloc', '--directions', 'both', '--kinds', 'cdylib',
                    '--cpu', 'native', '--strip-tool', shutil.which('strip')], env=env, check=True)
    output.rename(root / variant)
os.sched_setaffinity(0, {min(os.sched_getaffinity(0))})
for backend in ('auto', 'no-alloc'):
    output = root / ('native-' + backend + '.json')
    subprocess.run([sys.executable, str(source / 'docs/benchmarks/small-input-encoding/compare.py'),
                    str(root / 'reference' / (backend + '-both-cdylib.so')),
                    str(root / 'candidate' / (backend + '-both-cdylib.so')), str(output),
                    '16', '22', '31', '32', '33', '63', '64', '65', '128', '512'], check=True)
