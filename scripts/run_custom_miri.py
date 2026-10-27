#!/usr/bin/env python3
"""Run exact development Miri tests with fixed coverage and per-process buffers."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import time

from process_supervisor import install_signal_handlers, owned_process


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def skip_messages(text):
    return re.findall(
        r'^(skipped:[^\r\n]*|skipping[ \t]+[^\r\n]*|'
        r'no AVX2 on this CPU: AVX2 kernels? not run|'
        r'no AVX-512 VBMI on this CPU: VBMI kernels not run)$', text, re.M | re.I)


def source_files(source):
    files = [source / 'Cargo.toml', source / 'Cargo.lock']
    files += [path for folder in ['src', 'tests'] for path in (source / folder).rglob('*') if path.is_file()]
    return {str(path.relative_to(source)): digest(path) for path in files}


def sysroot_files(sysroot, target):
    directory = sysroot / 'lib/rustlib' / target
    require(directory.is_dir() and any(directory.rglob('libcore-*.rmeta')), 'explicit sysroot has no target core metadata')
    return {str(path.relative_to(sysroot)): digest(path) for path in directory.rglob('*') if path.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build-record', type=Path, required=True)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--arch', choices=['arm', 'x86'], required=True)
    parser.add_argument('--alias-model', choices=['stacked', 'tree'], default='tree')
    parser.add_argument('--suite', choices=['all', 'probes', 'kernels', 'buffers'], default='all')
    parser.add_argument('--seed', type=int, default=1)
    parser.add_argument('--cache', action='store_true')
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--sysroot', type=Path, help='reuse an explicit compatible sysroot without setup')
    parser.add_argument('--target-dir', type=Path, help='reuse an isolated Cargo target directory')
    parser.add_argument('--plan-only', action='store_true')
    parser.add_argument('--min-free-mib', type=int, default=2048)
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[1]
    coverage = json.loads((repository / 'ci/miri/coverage.json').read_text())
    pins = json.loads((repository / 'ci/miri/pins.json').read_text())
    record = json.loads(args.build_record.read_text())
    require(record['pins'] == pins and not record['prepared_only'], 'not a completed pinned custom build')
    tools = {name: Path(item['path']) for name, item in record['artifacts'].items()}
    for name in ['miri', 'cargo-miri']:
        require(digest(tools[name]) == record['artifacts'][name]['sha256'], 'custom tool hash mismatch: ' + name)
    require(0 <= args.seed < 2**32, 'seed must fit u32')
    require(args.min_free_mib >= 2048, 'the disk guard must retain at least 2 GiB')
    source, destination = args.source.resolve(), args.output.resolve()
    compiler = Path(record['compiler']).resolve()
    require(not destination.exists(), 'use a new output directory; old evidence is retained')
    require(source != destination and source not in destination.parents, 'output must be outside crate source')
    require(compiler != destination and compiler not in destination.parents, 'output must be outside compiler')
    if args.target_dir:
        target = args.target_dir.resolve()
        require(compiler != target and compiler not in target.parents, 'target directory must be outside compiler')
    require(shutil.disk_usage(destination.parent).free >= args.min_free_mib * 1024**2, 'insufficient free disk')
    destination.mkdir()
    snapshots = source_files(source)
    selected = coverage[args.arch]
    if args.suite in ['all', 'probes']:
        require(bool(selected['probes']) and len(selected['probes']) == len(set(selected['probes'])),
                'probe suite must be nonempty and unique')
    if args.suite in ['all', 'kernels']:
        for field in ['library_tests', 'integration_tests']:
            require(bool(selected[field]) and len(selected[field]) == len(set(selected[field])),
                    'kernel suite must be nonempty and unique: ' + field)
    env = os.environ.copy()
    for name in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTUP_TOOLCHAIN', 'RUSTC_WRAPPER',
                 'RUSTC_WORKSPACE_WRAPPER', 'RUSTC', 'RUSTDOC',
                 'MIRI', 'MIRI_BE_RUSTC', 'MIRI_CALLED_FROM_SETUP', 'MIRI_CALLED_FROM_RUSTDOC',
                 'MIRI_SYSROOT', 'MIRI_LIB_SRC', 'MIRIFLAGS']:
        env.pop(name, None)
    flags = ['-Zmiri-strict-provenance', '-Zmiri-seed=' + str(args.seed)]
    if args.alias_model == 'tree':
        flags.append('-Zmiri-tree-borrows')
    sysroot = args.sysroot.resolve() if args.sysroot else destination / 'sysroot'
    configured = dict(CARGO=record['cargo'], MIRI=str(tools['miri']),
        RUSTC=str(compiler / 'bin/rustc'), RUSTDOC=str(compiler / 'bin/rustdoc'),
        RUSTC_WRAPPER='', RUSTC_WORKSPACE_WRAPPER='',
        MIRI_SYSROOT=str(sysroot), MIRI_LIB_SRC=str(compiler / 'lib/rustlib/src/rust/library'),
        MIRIFLAGS=' '.join(flags), CARGO_ENCODED_RUSTFLAGS='\x1f'.join(shlex.split(selected['rustflags'])),
        CARGO_TARGET_DIR=str(args.target_dir.resolve() if args.target_dir else destination / 'target'),
        CARGO_BUILD_JOBS='1', CARGO_INCREMENTAL='0', CARGO_PROFILE_DEV_DEBUG='0', BASE58_REQUIRE_IFMA='1',
        DYLD_LIBRARY_PATH=str(compiler / 'lib'), LD_LIBRARY_PATH=str(compiler / 'lib'))
    env.update(configured)
    if args.offline:
        env['CARGO_NET_OFFLINE'] = 'true'
    compiler_version = subprocess.run([str(compiler / 'bin/rustc'), '-vV'],
        env=env, check=True, capture_output=True, text=True).stdout
    cargo_version = subprocess.run([record['cargo'], '-vV'],
        env=env, check=True, capture_output=True, text=True).stdout
    require('commit-hash: ' + pins['compiler_commit'] in compiler_version, 'actual compiler commit changed')
    require('commit-hash: ' + pins['cargo_commit'] in cargo_version, 'actual Cargo commit changed')
    toolchain_files = {str(path): digest(path) for path in [compiler / 'bin/rustc', Path(record['cargo'])]}
    sysroot_manifest = sysroot_files(sysroot, selected['target']) if args.sysroot else None
    base = [str(tools['cargo-miri']), 'miri', 'test', '--locked', '--target', selected['target'],
            '--features', selected['features'] + (',cache' if args.cache else '')]
    commands = []
    if not args.sysroot:
        commands.append(('setup', [str(tools['cargo-miri']), 'miri', 'setup', '--target', selected['target']], None))
    if args.suite in ['all', 'probes']:
        commands.append(('discover-probes', base + ['--test', 'miri_intrinsics', '--', '--list'], 'discover-probes'))
        commands += [(name, base + ['--test', 'miri_intrinsics', name, '--', '--exact'], 'one') for name in selected['probes']]
    if args.suite in ['all', 'kernels']:
        commands += [(name, base + ['--lib', name, '--', '--exact'], 'one') for name in selected['library_tests']]
        commands += [(name, base + ['--test', 'miri', name, '--', '--exact'], 'one') for name in selected['integration_tests']]
    if args.suite in ['all', 'buffers']:
        cases = coverage['buffer_cases']
        require(len(cases) == len(set(cases)) == 14, 'expected all 14 original buffer cases')
        commands.append(('discover-buffers', base + ['--test', 'miri', '--', '--list'], 'discover'))
        commands += [(name, base + ['--test', 'miri', name, '--', '--ignored', '--exact'], 'one') for name in cases]
    require(any(expected == 'one' for _, _, expected in commands), 'requested suite selected no tests')
    for _, command, expected in commands:
        if expected == 'one':
            # Expose successful-test output so an ISA-skip early return cannot hide.
            command.append('--show-output')
    receipt = dict(build_record_sha256=digest(args.build_record), pins=pins, arch=args.arch,
        actual_compiler_version=compiler_version, actual_cargo_version=cargo_version, toolchain_files=toolchain_files,
        alias_model=args.alias_model, seed=args.seed, suite=args.suite, environment=configured,
        coverage_sha256=digest(repository / 'ci/miri/coverage.json'), source_files=snapshots,
        plan_only=args.plan_only, commands=[], planned_commands=[item[1] for item in commands])
    receipt['sysroot'] = dict(path=str(sysroot), external=args.sysroot is not None,
        manifest_scope='all files under lib/rustlib/' + selected['target'], files=sysroot_manifest)
    (destination / 'run.json').write_text(json.dumps(receipt, indent=2) + '\n')
    if args.plan_only:
        return
    for index, (name, command, expected) in enumerate(commands):
        require(shutil.disk_usage(destination).free >= args.min_free_mib * 1024**2, 'insufficient free disk')
        require(source_files(source) == snapshots, 'crate source changed during the run')
        for tool in ['miri', 'cargo-miri']:
            require(digest(tools[tool]) == record['artifacts'][tool]['sha256'], 'custom tool changed during the run')
        print('Miri ' + name, flush=True)
        log = destination / f'{index:02}.log'
        start = time.monotonic()
        guard = None
        with log.open('x') as stream:
            with owned_process(command, cwd=source, env=env, stdout=stream,
                               stderr=subprocess.STDOUT) as process:
                while process.poll() is None:
                    if shutil.disk_usage(destination).free < args.min_free_mib * 1024**2:
                        guard = 'free_disk_below_floor'
                    elif time.monotonic() - start > 1200:
                        guard = 'test_deadline'
                    if guard:
                        break
                    time.sleep(2)
        text = log.read_text()
        passed = process.returncode == 0 and guard is None
        skipped = skip_messages(text) if expected == 'one' else []
        if expected == 'one':
            passed = passed and len(re.findall(r'^test result: ok\. 1 passed; 0 failed; 0 ignored;', text, re.M)) == 1
            passed = passed and not skipped
        elif expected == 'discover-probes':
            found = re.findall(r'^([^\s]+): test$', text, re.M)
            passed = passed and len(found) == len(selected['probes']) and set(found) == set(selected['probes'])
        elif expected == 'discover':
            found = re.findall(r'^(buffer_cases::[^\s:]+): test$', text, re.M)
            passed = passed and len(found) == 14 and set(found) == set(coverage['buffer_cases'])
        passed = passed and source_files(source) == snapshots
        passed = passed and all(digest(Path(path)) == expected for path, expected in toolchain_files.items())
        if passed and name == 'setup':
            sysroot_manifest = sysroot_files(sysroot, selected['target'])
            receipt['sysroot']['files'] = sysroot_manifest
        receipt['commands'].append(dict(name=name, expectation=expected, argv=command, passed=passed, exit_code=process.returncode,
            skip_messages=skipped, guard=guard, log_sha256=digest(log), elapsed_seconds=time.monotonic()-start))
        (destination / 'run.json').write_text(json.dumps(receipt, indent=2) + '\n')
        require(passed, 'Miri command failed or selected coverage was missing: ' + str(log))
    require(sysroot_files(sysroot, selected['target']) == sysroot_manifest, 'Miri sysroot changed during the run')
    receipt['sysroot']['unchanged_after_tests'] = True
    receipt['passed'] = True
    (destination / 'run.json').write_text(json.dumps(receipt, indent=2) + '\n')


if __name__ == '__main__':
    install_signal_handlers()
    main()
