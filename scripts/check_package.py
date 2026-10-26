#!/usr/bin/env python3
"""Check that the distributable crate contains source/docs, not research artifacts."""
import argparse
import json
from pathlib import Path, PurePosixPath
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
EXACT = {'Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock', '.cargo_vcs_info.json',
         'README.md', 'DESIGN.md', 'CORRECTNESS.md', 'REFERENCES.md',
         'LICENSE-MIT', 'LICENSE-APACHE', 'docs/README.md', 'docs/performance.md',
         'docs/assembly.md', 'docs/simd.md', 'docs/verification/README.md',
         'docs/verification/local-ci.md', 'docs/verification/local-ci.json',
         'docs/verification/proofs.json', 'docs/verification/sanitizers.json',
         'docs/verification/status.json', 'docs/references/consultations.json',
         'docs/references/firedancer-LICENSE.txt', 'docs/references/firedancer-NOTICE.txt',
         'docs/references/bitcoin-core-LICENSE.txt',
         'docs/references/five8-LICENSE.txt'}
REQUIRED = {'Cargo.toml', 'src/lib.rs', 'README.md', 'DESIGN.md', 'CORRECTNESS.md',
            'REFERENCES.md', 'LICENSE-MIT', 'LICENSE-APACHE'}

def require(condition, message):
    if not condition:
        raise ValueError(message)

def allowed(name):
    path = PurePosixPath(name)
    if not path.parts or path.is_absolute() or '..' in path.parts:
        return False
    if name in EXACT:
        return True
    if path.parts[0] in {'src', 'tests', 'benches'}:
        return path.suffix == '.rs'
    if path.parent == PurePosixPath('assets/graphs'):
        return path.suffix == '.svg'
    return False

def self_test():
    for name in ['src/lib.rs', 'tests/support/scalar.rs', 'REFERENCES.md', 'CORRECTNESS.md',
                 'assets/graphs/encode.svg', 'docs/verification/proofs.json']:
        require(allowed(name), 'Expected package source: ' + name)
    for name in ['', '../secret.rs', '/src/lib.rs', 'src/key.pem', 'fuzz/corpus/input',
                 'docs/benchmarks/source.tgz', 'docs/verification/run.log',
                 'target/release/lib.rlib', '.git/config']:
        require(not allowed(name), 'Unexpected package input accepted: ' + name)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path, nargs='?', default=ROOT / 'target/package')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    self_test()
    if args.self_test:
        print('Package policy self-check passes')
        return
    package = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']
    require(package['name'] == 'faster-base58', 'Unexpected package identity')
    prefix = package['name'] + '-' + package['version']
    archive = args.directory / (prefix + '.crate')
    names, total = set(), 0
    with tarfile.open(archive, 'r:gz') as stream:
        for member in stream:
            require(member.isfile(), 'Non-file package member: ' + member.name)
            path = PurePosixPath(member.name)
            require(len(path.parts) > 1 and path.parts[0] == prefix, 'Wrong package root')
            name = str(PurePosixPath(*path.parts[1:]))
            require(name not in names and allowed(name), 'Unexpected package member: ' + name)
            names.add(name)
            total += member.size
            require(total <= 2 * 1024 * 1024, 'Package source exceeds the 2 MiB budget')
            if name == 'Cargo.toml':
                manifest = tomllib.loads(stream.extractfile(member).read().decode())
    require(REQUIRED <= names, 'Missing package files: ' + str(REQUIRED - names))
    require(manifest['package']['license'] == 'MIT OR Apache-2.0', 'Wrong license metadata')
    require(manifest['features']['default'] == [] and 'std' in manifest['features']['cache'],
            'Default/cache feature policy differs')
    require('malachite-nz' not in manifest.get('dependencies', {}), 'Development oracle leaked into dependencies')
    print(json.dumps(dict(status='pass', archive=str(archive), files=len(names),
                          uncompressed_bytes=total, license=manifest['package']['license']), indent=2))

if __name__ == '__main__':
    main()
