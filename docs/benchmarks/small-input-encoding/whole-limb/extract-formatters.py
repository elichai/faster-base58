"""Extract PR1 and current whole-limb formatters, then inspect both targets."""
from pathlib import Path
import subprocess
import sys

root = Path('/tmp/base58-pr1-review')
parts = ['const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";\nconst BASE: u64 = 58u64.pow(10);']
for name in ('pr', 'candidate'):
    def read(relative):
        if name == 'pr':
            return subprocess.check_output(['git', 'show', '053f00810de3f39b03f2891a2771348e67bfa4e6:' + relative], cwd=root, text=True)
        return (root / relative).read_text()
    source = read('src/encode/mod.rs')
    body = source[source.index('fn ten_digits('):source.index('#[cfg(kani)]')]
    body = body.replace('fn ten_digits(', 'pub fn ten_digits(')
    matrix = read('src/encode/matrix.rs')
    table = matrix[matrix.index('pub(super) fn four_digits('):matrix.index('#[cfg(test)]\nmod tests')]
    table = table.replace('const CHAR_PAIRS:', 'pub(super) const CHAR_PAIRS:').replace('pub(super) pub(super)', 'pub(super)')
    parts.append(f'mod {name} {{ use super::{{ALPHABET, BASE}}; mod matrix {{ use super::ALPHABET; {table} }} {body} }}')
    parts.append(f'#[unsafe(no_mangle)]\n#[inline(never)]\npub fn format_{name}(n: u64) -> [u8; 10] {{ {name}::ten_digits(n) }}')
suffix = sys.argv[1] if len(sys.argv) > 1 else ''
target = Path('/tmp/base58_pr1_ten_codegen' + suffix + '.rs')
assert not target.exists()
target.write_text('\n'.join(parts))
for arch in ('aarch64-apple-darwin', 'x86_64-unknown-linux-gnu'):
    subprocess.run(['rustc', '--edition=2024', '--crate-type=lib', '-O', '--emit=asm',
                    '--target', arch, '-A', 'dead_code', '-A', 'unused_imports', str(target),
                    '-o', f'/tmp/base58_pr1_ten{suffix}_{arch}.s'], check=True)
