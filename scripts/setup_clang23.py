#!/usr/bin/env python3
"""Extract pinned official Clang packages into a fresh RUNNER_TEMP prefix."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import urllib.request

BASE = 'https://apt.llvm.org/bookworm/'
NAMES = {'clang-23', 'libclang-cpp23', 'libllvm23', 'libclang-common-23-dev',
         'libclang1-23', 'libclang-rt-23-dev', 'llvm-23-linker-tools'}


def self_test(packages):
    for package in packages:
        validate_package(package)
    rejected = 0
    for filename in ['../escape.deb', '/usr/lib/escape.deb',
                     'pool/main/l/llvm-toolchain-23/../escape_amd64.deb',
                     'pool/main/l/llvm-toolchain-23/%2e%2e_amd64.deb']:
        bad = dict(packages[0], Filename=filename, URL=BASE + filename)
        try:
            validate_package(bad)
        except ValueError:
            rejected += 1
    assert rejected == 4, 'unsafe Filename was accepted'
    try:
        validate_package(dict(packages[0], URL=packages[0]['URL'].replace('https:', 'http:')))
    except ValueError:
        pass
    else:
        raise AssertionError('non-HTTPS URL accepted')
    try:
        verify_payload(b'payload', 7, '0' * 64)
    except ValueError:
        pass
    else:
        raise AssertionError('wrong hash accepted')
    verify_payload(b'payload', 7, hashlib.sha256(b'payload').hexdigest())
    temporary = Path(tempfile.mkdtemp(prefix='clang23-selftest-'))
    prefix = temporary / 'clang'
    create_prefix(prefix, temporary)
    try:
        create_prefix(prefix, temporary)
    except FileExistsError:
        pass
    else:
        raise AssertionError('prefix reuse accepted')
    link = temporary / 'link'
    link.symlink_to(temporary / 'missing')
    try:
        create_prefix(link, temporary)
    except FileExistsError:
        pass
    else:
        raise AssertionError('existing symlink prefix accepted')
    try:
        create_prefix(temporary.parent / (temporary.name + '-outside'), temporary)
    except ValueError:
        pass
    else:
        raise AssertionError('prefix outside RUNNER_TEMP accepted')
    print('setup_clang23 self-test: pass')


def validate_package(package):
    filename = package['Filename']
    parts = filename.split('/')
    if (package['Package'] not in NAMES or len(parts) != 5
            or parts[:4] != ['pool', 'main', 'l', 'llvm-toolchain-23']
            or not re.fullmatch(r'[A-Za-z0-9_+.~:-]+_amd64\.deb', parts[-1])
            or not parts[-1].startswith(package['Package'] + '_')):
        raise ValueError('unsafe Filename: ' + filename)
    if package['URL'] != BASE + filename:
        raise ValueError('URL must be the exact official HTTPS package URL')
    if not re.fullmatch(r'[0-9a-f]{64}', package['SHA256']):
        raise ValueError('invalid SHA256')
    if not 0 < int(package['Size']) <= 100 * 1024 * 1024:
        raise ValueError('invalid package size')


def verify_payload(data, size, expected):
    if len(data) != size or hashlib.sha256(data).hexdigest() != expected:
        raise ValueError('package size/SHA256 mismatch')


def create_prefix(prefix, runner_temp):
    if prefix.exists() or prefix.is_symlink():
        raise FileExistsError(prefix)
    prefix, runner_temp = prefix.resolve(), runner_temp.resolve()
    if prefix == runner_temp or not prefix.is_relative_to(runner_temp):
        raise ValueError('prefix must be inside RUNNER_TEMP')
    prefix.mkdir(parents=True, exist_ok=False)
    return prefix


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prefix', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    manifest = json.loads(Path(__file__).with_name('clang23-packages.json').read_text())
    if args.self_test:
        self_test(manifest['packages'])
        return
    packages = manifest['packages']
    if len(packages) != len(NAMES) or {p['Package'] for p in packages} != NAMES:
        raise ValueError('expected all seven pinned packages once')
    for package in packages:
        validate_package(package)
    runner_temp = Path(os.environ['RUNNER_TEMP'])
    prefix = create_prefix(args.prefix or runner_temp / 'clang23', runner_temp)
    downloads = prefix / 'downloads'
    downloads.mkdir()
    for package in packages:
        print('Extracting ' + package['Package'], flush=True)
        size = int(package['Size'])
        with urllib.request.urlopen(package['URL'], timeout=120) as response:
            if response.geturl() != package['URL']:
                raise ValueError('package redirect is not allowed')
            data = response.read(size + 1)
        verify_payload(data, size, package['SHA256'])
        archive = downloads / Path(package['Filename']).name
        with archive.open('xb') as output:
            output.write(data)
        subprocess.run(['dpkg-deb', '--extract', archive, prefix], check=True)
    clang = prefix / 'usr/lib/llvm-23/bin/clang'
    clangxx = clang.with_name('clang++')
    libraries = str(prefix / 'usr/lib/x86_64-linux-gnu')
    if os.environ.get('LD_LIBRARY_PATH'):
        libraries += ':' + os.environ['LD_LIBRARY_PATH']
    values = {'CC': str(clang), 'CXX': str(clangxx),
              'CC_x86_64-unknown-linux-gnu': str(clang),
              'CXX_x86_64-unknown-linux-gnu': str(clangxx), 'LD_LIBRARY_PATH': libraries}
    env = dict(os.environ, **values)
    version = subprocess.check_output([clangxx, '--version'], env=env, text=True)
    rust = subprocess.check_output(['rustup', 'run', 'nightly-2026-10-26', 'rustc', '-vV'], env=env, text=True)
    if not version.startswith('Debian clang version 23.1.5 ') or 'LLVM version: 23.' not in rust:
        raise ValueError('expected pinned Clang23.1.5 and Rust LLVM23')
    if os.environ.get('GITHUB_ENV'):
        with Path(os.environ['GITHUB_ENV']).open('a') as output:
            for key, value in values.items():
                output.write(key + '=' + value + '\n')
    print(json.dumps({'clang': version, 'rustc': rust, 'environment': values}, indent=2))


if __name__ == '__main__':
    main()
