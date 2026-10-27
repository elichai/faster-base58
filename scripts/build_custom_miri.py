#!/usr/bin/env python3
"""Build the reviewed development interpreter without changing installed toolchains."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile
import time

from process_supervisor import install_signal_handlers, owned_process


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def output(args, cwd=None, env=None):
    return subprocess.run(args, cwd=cwd, env=env, check=True,
                          capture_output=True, text=True).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True, help='clean checkout of the pinned Miri commit')
    parser.add_argument('--compiler', type=Path, required=True, help='matching compiler sysroot directory')
    parser.add_argument('--cargo', type=Path, required=True, help='pinned Cargo executable, not a rustup proxy')
    parser.add_argument('--output', type=Path, required=True, help='new private build directory')
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--min-free-mib', type=int, default=2048)
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[1]
    pins = json.loads((repository / 'ci/miri/pins.json').read_text())
    patch = repository / 'ci/miri/shims.patch'
    require(digest(patch) == pins['patch_sha256'], 'Miri patch checksum mismatch')
    source, compiler, cargo = args.source.resolve(), args.compiler.resolve(), args.cargo.resolve()
    destination = args.output.resolve()
    require(sys.platform in ['darwin', 'linux'], 'custom Miri builder supports macOS and Linux')
    require(cargo.name != 'rustup', 'pass the executable returned by rustup which cargo')
    require(source != destination and source not in destination.parents, 'output must be outside Miri source')
    require(compiler != destination and compiler not in destination.parents, 'output must be outside compiler')
    require(not destination.exists(), 'use a fresh output directory; old evidence is retained')
    require(output(['git', 'rev-parse', 'HEAD'], source).strip() == pins['miri_commit'], 'wrong Miri commit')
    require(not output(['git', 'status', '--porcelain', '--untracked-files=no'], source), 'Miri source is dirty')
    require((source / 'rust-version').read_text().strip() == pins['compiler_commit'], 'wrong required compiler')
    compiler_version = output([str(compiler / 'bin/rustc'), '-vV'])
    cargo_version = output([str(cargo), '-vV'])
    require('commit-hash: ' + pins['compiler_commit'] in compiler_version, 'wrong compiler commit')
    require('commit-hash: ' + pins['cargo_commit'] in cargo_version, 'wrong Cargo commit')
    host = next(line[6:] for line in compiler_version.splitlines() if line.startswith('host: '))
    require(args.min_free_mib >= 2048, 'the disk guard must retain at least 2 GiB')
    require(shutil.disk_usage(destination.parent).free >= args.min_free_mib * 1024**2, 'insufficient free disk')
    destination.mkdir()
    work = destination / 'miri'
    work.mkdir()
    env = os.environ.copy()
    for name in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTUP_TOOLCHAIN', 'RUSTC_WRAPPER',
                 'RUSTC_WORKSPACE_WRAPPER', 'RUSTC', 'RUSTDOC',
                 'MIRI', 'MIRI_BE_RUSTC', 'MIRI_CALLED_FROM_SETUP', 'MIRI_CALLED_FROM_RUSTDOC',
                 'MIRI_SYSROOT', 'MIRI_LIB_SRC', 'MIRIFLAGS']:
        env.pop(name, None)
    for name in ['GIT_DIR', 'GIT_WORK_TREE', 'GIT_COMMON_DIR', 'GIT_INDEX_FILE']:
        env.pop(name, None)
    env['GIT_NO_REPLACE_OBJECTS'] = '1'
    expected_blobs = {}
    for item in output(['git', 'ls-tree', '-r', '-z', pins['miri_commit']], source, env).split('\0'):
        if item:
            metadata, name = item.split('\t', 1)
            mode, kind, object_id = metadata.split()
            if kind == 'blob':
                require(mode in ['100644', '100755'], 'Miri tree contains an unsupported file type')
                expected_blobs[name] = object_id
    archive = subprocess.run(['git', 'archive', '--format=tar', pins['miri_commit']],
        cwd=source, env=env, check=True, capture_output=True).stdout
    names = []
    with tarfile.open(fileobj=io.BytesIO(archive)) as tree:
        for entry in tree:
            name = PurePosixPath(entry.name)
            require(not name.is_absolute() and '..' not in name.parts, 'invalid archived path')
            if entry.isdir():
                continue
            require(entry.isfile(), 'archived symlink or special file requires explicit handling')
            with tree.extractfile(entry) as stream:
                data = stream.read()
            object_id = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
            require(expected_blobs.pop(str(name), None) == object_id, 'archive differs from pinned Git tree')
            target = work / str(name)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            target.chmod(0o755 if entry.mode & 0o111 else 0o644)
            names.append(str(name))
    require(not expected_blobs, 'archive omitted pinned source files')
    require(not output(['git', 'status', '--porcelain', '--untracked-files=no'], source), 'Miri source changed')
    # Prevent git apply/build metadata from discovering an unrelated ancestor checkout.
    env['GIT_CEILING_DIRECTORIES'] = str(destination)
    subprocess.run(['git', 'apply', '--check', str(patch)], cwd=work, env=env, check=True)
    subprocess.run(['git', 'apply', str(patch)], cwd=work, env=env, check=True)
    files = {name: digest(work / name) for name in names}
    (destination / 'source.json').write_text(json.dumps(files, indent=2) + '\n')
    flags = ['-L', 'native=' + str(compiler / 'lib'), '-C',
             'link-arg=-Wl,-rpath,' + str(compiler / 'lib'), '-C',
             'link-arg=-Wl,-rpath,' + str(compiler / 'lib/rustlib' / host / 'lib'),
             '-Zunstable-options', '-Wrustc::internal', '-Wrust_2018_idioms', '-Wunused_lifetimes']
    selected = dict(CARGO=str(cargo), RUSTC=str(compiler / 'bin/rustc'),
        RUSTDOC=str(compiler / 'bin/rustdoc'), CARGO_TARGET_DIR=str(destination / 'target'),
        RUSTC_WRAPPER='', RUSTC_WORKSPACE_WRAPPER='',
        CARGO_BUILD_JOBS='1', CARGO_INCREMENTAL='0', CARGO_PROFILE_DEV_DEBUG='0',
        CARGO_PROFILE_DEV_OPT_LEVEL='1', CARGO_ENCODED_RUSTFLAGS='\x1f'.join(flags))
    env.update(selected)
    if args.offline:
        env['CARGO_NET_OFFLINE'] = 'true'
    receipt = dict(pins=pins, source=str(source), compiler=str(compiler), cargo=str(cargo),
        source_archive_sha256=hashlib.sha256(archive).hexdigest(),
        compiler_version=compiler_version, cargo_version=cargo_version, environment=selected,
        prepared_only=args.prepare_only, build_commands=[])
    if not args.prepare_only:
        commands = [
            [str(cargo), 'build', '--locked', '--no-default-features', '--features',
             'stack-cache', '--bin', 'miri', '--manifest-path', str(work / 'Cargo.toml')],
            [str(cargo), 'build', '--locked', '--bin', 'cargo-miri', '--manifest-path',
             str(work / 'cargo-miri/Cargo.toml')],
        ]
        for index, command in enumerate(commands):
            require(shutil.disk_usage(destination).free >= args.min_free_mib * 1024**2, 'insufficient free disk')
            print('Building ' + ['interpreter', 'wrapper'][index], flush=True)
            log = destination / f'build-{index}.log'
            start = time.monotonic()
            guard = None
            with log.open('x') as stream:
                with owned_process(command, cwd=work, env=env, stdout=stream,
                                   stderr=subprocess.STDOUT) as process:
                    while process.poll() is None:
                        if shutil.disk_usage(destination).free < args.min_free_mib * 1024**2:
                            guard = 'free_disk_below_floor'
                        elif time.monotonic() - start > 900:
                            guard = 'build_deadline'
                        if guard:
                            break
                        time.sleep(2)
            for name, expected in files.items():
                require(digest(work / name) == expected, 'Miri source changed during build: ' + name)
            receipt['build_commands'].append(dict(argv=command, exit_code=process.returncode,
                guard=guard, log_sha256=digest(log), elapsed_seconds=time.monotonic()-start))
            (destination / 'build.json').write_text(json.dumps(receipt, indent=2) + '\n')
            if guard or process.returncode:
                raise SystemExit('custom Miri build failed; inspect ' + str(log))
        receipt['artifacts'] = {name: dict(path=str(destination / 'target/debug' / name),
            sha256=digest(destination / 'target/debug' / name)) for name in ['miri', 'cargo-miri']}
    (destination / 'build.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print('Custom Miri receipt: ' + str(destination / 'build.json'))


if __name__ == '__main__':
    install_signal_handlers()
    main()
