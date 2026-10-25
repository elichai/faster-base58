#!/usr/bin/env python3
"""Check the normal API and the feature-gated internals namespace downstream."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
NAMES = [f'{direction}_{backend}' for direction in ['encode', 'decode']
         for backend in ['batched', 'karatsuba', 'matrix', 'matrix_portable',
                         'ntt', 'ntt_cached', 'ntt_portable', 'toom3']]
HOOKS = ['Multiplier', 'Leaves', 'decode_with', 'encode_tuned', 'ifma_threshold',
         'mul_with', 'scratch_len', 'encode_karatsuba_with', 'encode_matrix_with',
         'encode_matrix_whole', 'encode_matrix_cache_memory', 'fixed_matrix_table_bytes',
         'decode_matrix_cache_memory', 'decode_matrix_forced', 'decode_ntt_with',
         'encode_ntt_with', 'decode_ntt_same_leaves', 'encode_ntt_same_leaves',
         'ntt_cache_memory', 'decode_toom3_with']
PROFILES = [('default', []), ('cache', ['cache']), ('internals', ['__internals']),
            ('cache-internals', ['cache', '__internals'])]

NORMAL = r'''
use base58_bench::{encode, decode, encode_into, decode_into, encode_string, decode_vec,
                  decode_array, max_encoded_len, BufferTooSmall, Error, no_alloc};
fn main() {
    let input = [0, 0, 1];
    let mut text = [0xA5; 16];
    let mut bytes = [0xA5; 16];
    assert_eq!(encode(&input, &mut text), Ok(3));
    assert_eq!(&text[..3], b"112");
    assert!(text[3..].iter().all(|&b| b == 0xA5));
    assert_eq!(decode("112", &mut bytes), Ok(3));
    assert_eq!(&bytes[..3], &input);
    assert!(bytes[3..].iter().all(|&b| b == 0xA5));
    assert_eq!(encode_into(input, &mut text), Ok("112"));
    assert_eq!(decode_into("112", &mut bytes).unwrap(), input);
    assert_eq!(encode_string(input), "112");
    assert_eq!(decode_vec("112"), Ok(input.to_vec()));
    assert_eq!(decode_array::<3>("112"), Ok(input));
    assert_eq!(decode_array::<4>("112"), Err(Error::InvalidLength { expected: 4, actual: 3 }));
    assert_eq!(decode_array::<2>("112"), Err(Error::BufferTooSmall));
    assert_eq!(max_encoded_len(usize::MAX), usize::MAX);
    assert_eq!(encode(&input, &mut []), Err(BufferTooSmall));
    assert_eq!(decode("1é", &mut bytes), Err(Error::InvalidCharacter { index: 1 }));
    let _: fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall> = no_alloc::encode;
    let _: fn(&str, &mut [u8]) -> Result<usize, Error> = no_alloc::decode;
    let _: for<'a, 'b> fn(&'a [u8], &'b mut [u8]) -> Result<&'b str, BufferTooSmall> = no_alloc::encode_into;
    let _: for<'a, 'b> fn(&'a str, &'b mut [u8]) -> Result<&'b mut [u8], Error> = no_alloc::decode_into;
    assert_eq!(no_alloc::encode(&input, &mut text), Ok(3));
    assert_eq!(no_alloc::decode("112", &mut bytes), Ok(3));
    let owned = input.to_vec();
    let borrowed = no_alloc::encode_into(&owned, &mut text).unwrap();
    drop(owned);
    assert_eq!(borrowed, "112");
    assert_eq!(no_alloc::decode_into("112", &mut bytes).unwrap(), input);
    assert_eq!(no_alloc::decode_array::<0>(""), Ok([]));
    assert_eq!(no_alloc::decode_array::<3>("112"), Ok(input));
    assert_eq!(no_alloc::decode_array::<2>("112"), Err(Error::BufferTooSmall));
    assert_eq!(no_alloc::decode_array::<4>("112"), Err(Error::InvalidLength { expected: 4, actual: 3 }));
    assert_eq!(no_alloc::decode("1é", &mut bytes), Err(Error::InvalidCharacter { index: 1 }));
}
'''

INTERNAL = r'''
#![allow(unused_imports)]
use base58_bench::{BufferTooSmall, Error};
use base58_bench::internals::*;
use base58_bench::internals::{HOOK_NAMES};
#[cfg(target_arch = "x86_64")]
use base58_bench::internals::{set_ifma_threshold, ifma_mul, ifma_scratch_len};
fn main() {
    type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;
    type Decode = fn(&str, &mut [u8]) -> Result<usize, Error>;
    let encoders: [Encode; 8] = [encode_batched, encode_karatsuba, encode_matrix, encode_matrix_portable,
                               encode_ntt, encode_ntt_cached, encode_ntt_portable, encode_toom3];
    let decoders: [Decode; 8] = [decode_batched, decode_karatsuba, decode_matrix, decode_matrix_portable,
                               decode_ntt, decode_ntt_cached, decode_ntt_portable, decode_toom3];
    let _: Option<&dyn Multiplier> = None;
    let _ = (Leaves::Horner, Leaves::Matrix);
    for encode in encoders {
        let mut output = [0xA5; 8];
        assert_eq!(encode(&[0, 0, 1], &mut output), Ok(3));
        assert_eq!(&output[..3], b"112");
        assert_eq!(&output[3..], &[0xA5; 5]);
    }
    for decode in decoders {
        let mut output = [0xA5; 8];
        assert_eq!(decode("112", &mut output), Ok(3));
        assert_eq!(&output[..3], &[0, 0, 1]);
        assert_eq!(&output[3..], &[0xA5; 5]);
    }
}
'''.replace('HOOK_NAMES', ', '.join(HOOKS))


def fingerprint(source):
    paths = [source / 'Cargo.toml', source / 'Cargo.lock']
    paths += list((source / 'src').rglob('*.rs')) + list((source / 'tests/support').rglob('*.rs'))
    digest = hashlib.sha256()
    for path in sorted(paths):
        digest.update(str(path.relative_to(source)).encode() + b'\0' + path.read_bytes())
    return digest.hexdigest()


def run(command, label, source, output, environment):
    prefix = ['rtk', 'proxy'] if shutil.which('rtk') else []
    result = subprocess.run([*prefix, *map(str, command)], cwd=source, env=environment,
                            capture_output=True, text=True)
    (output / (label + '.stdout')).write_text(result.stdout)
    (output / (label + '.stderr')).write_text(result.stderr)
    return result


def build_environment():
    environment = dict(os.environ, RUSTFLAGS='', CARGO_INCREMENTAL='0')
    for key in ['CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_RUSTFLAGS',
                'CARGO_BUILD_TARGET', 'CARGO_TARGET_DIR', 'CARGO_BUILD_TARGET_DIR']:
        environment.pop(key, None)
    return environment


def self_test(source, output):
    from unittest.mock import patch

    stdout = '{"reason":"compiler-artifact"}\n'
    stderr = '{"level":"error","code":{"code":"E0432"},"message":"unresolved import"}\n'
    command = ['rustc', '--error-format=json']
    for available in [False, True]:
        completed = subprocess.CompletedProcess(command, 1, stdout, stderr)
        with patch.object(shutil, 'which', return_value='/test/rtk' if available else None):
            with patch.object(subprocess, 'run', return_value=completed) as child:
                actual = run(command, 'rtk' if available else 'direct', source, output, {})
        expected = ['rtk', 'proxy', *command] if available else command
        assert child.call_args.args == (expected,)
        assert 'shell' not in child.call_args.kwargs
        assert actual.stdout == stdout and actual.stderr == stderr
        assert json.loads(actual.stderr)['code']['code'] == 'E0432'
    poison = {name: 'poison' for name in ['CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_RUSTFLAGS',
              'CARGO_BUILD_TARGET', 'CARGO_TARGET_DIR', 'CARGO_BUILD_TARGET_DIR']}
    with patch.dict(os.environ, {**poison, 'RUSTUP_TOOLCHAIN': 'retained-toolchain'}, clear=True):
        environment = build_environment()
    assert not set(poison) & set(environment)
    assert environment['RUSTUP_TOOLCHAIN'] == 'retained-toolchain'
    (output / 'self-test.json').write_text(json.dumps(dict(status='pass',
        checks=['direct subprocess without RTK', 'optional RTK prefix', 'JSON diagnostics unchanged',
                'host target/output environment isolated', 'user toolchain retained']), indent=2) + '\n')
    print('PASS public-API checker self-test; no compiler or worker ran')


def check_fixture(name, source_file, profile, rlib, source, output, environment, expected=None):
    destination = output / profile / name
    label = profile + '-' + name
    built = run(['rustc', '--edition=2024', '--error-format=json', '--crate-name', name,
                 source_file, '--extern', 'base58_bench=' + str(rlib),
                 '-L', 'dependency=' + str(rlib.parent / 'deps'), '-o', destination],
                label, source, output, environment)
    if expected:
        code, subject = expected
        diagnostics = [json.loads(line) for line in built.stderr.splitlines() if line.startswith('{')]
        assert built.returncode == 1 and any(row.get('level') == 'error'
            and (row.get('code') or {}).get('code') == code and subject in row.get('message', '')
            for row in diagnostics), (label, built.stderr)
    else:
        assert built.returncode == 0, (label, built.stderr)
        result = run([destination], label + '-run', source, output, environment)
        assert result.returncode == 0, (label, result.stderr)
    return dict(fixture=name, expected_error=expected, returncode=built.returncode)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT)
    parser.add_argument('--output', type=Path, required=True, help='new scratch directory')
    parser.add_argument('--self-test', action='store_true', help='check process/JSON/environment handling without compiling')
    args = parser.parse_args()
    assert __debug__, 'do not use Python -O: checks use assertions'
    source, output = args.source.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    if args.self_test:
        self_test(source, output)
        return
    before = fingerprint(source)
    fixtures = {'normal': NORMAL, 'internals_positive': INTERNAL,
                'root_bench': 'use base58_bench::bench; fn main() { let _ = bench::ifma_threshold; }\n',
                'private_encode': 'use base58_bench::encode::encode_batched; fn main() { let _ = encode_batched; }\n',
                'absent_internals': 'use base58_bench::internals; fn main() { let _ = internals::ifma_threshold; }\n',
                'owned_input': 'fn main() { let mut out = [0; 8]; let _ = base58_bench::no_alloc::encode_into(vec![1], &mut out); }\n'}
    fixtures.update({'root_' + name: f'use base58_bench::{name}; fn main() {{ let _ = {name}; }}\n' for name in NAMES})
    for name, content in fixtures.items():
        (output / (name + '.rs')).write_text(content)
    environment = build_environment()
    profiles = []
    for profile, features in PROFILES:
        (output / profile).mkdir()
        target = output / profile / 'target'
        command = ['cargo', 'build', '--locked', '--lib', '--manifest-path', source / 'Cargo.toml',
                   '--target-dir', target, '--message-format=json']
        if features:
            command += ['--features', ','.join(features)]
        build = run(command, profile + '-build', source, output, environment)
        assert build.returncode == 0, (profile, build.stderr)
        messages = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
        artifacts = [row for row in messages if row.get('reason') == 'compiler-artifact' and row['target']['name'] == 'base58_bench']
        assert len(artifacts) == 1 and set(artifacts[0]['features']) == set(features)
        assert artifacts[0]['profile']['test'] is False
        rlib = target / 'debug/libbase58_bench.rlib'
        checks = [check_fixture('normal', output / 'normal.rs', profile, rlib, source, output, environment)]
        for name in NAMES:
            checks.append(check_fixture('root_' + name, output / ('root_' + name + '.rs'), profile,
                                        rlib, source, output, environment, ('E0432', name)))
        checks.append(check_fixture('root_bench', output / 'root_bench.rs', profile, rlib, source, output, environment, ('E0432', 'bench')))
        checks.append(check_fixture('private_encode', output / 'private_encode.rs', profile, rlib, source, output, environment, ('E0603', 'encode')))
        checks.append(check_fixture('owned_input', output / 'owned_input.rs', profile, rlib, source, output, environment, ('E0308', 'mismatched types')))
        if '__internals' in features:
            checks.append(check_fixture('internals_positive', output / 'internals_positive.rs', profile, rlib, source, output, environment))
        else:
            checks.append(check_fixture('absent_internals', output / 'absent_internals.rs', profile, rlib, source, output, environment, ('E0432', 'internals')))
        profiles.append(dict(profile=profile, features=features, checks=checks,
                             rlib_sha256=hashlib.sha256(rlib.read_bytes()).hexdigest()))
        print('PASS', profile, len(checks), 'downstream checks', flush=True)
    assert fingerprint(source) == before, 'library/support sources changed during validation'
    receipt = dict(status='pass', created_utc=datetime.now(timezone.utc).isoformat(), source_sha256=before,
                   checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                   root_backend_rejections=64, root_bench_rejections=4, private_encode_rejections=4,
                   absent_internals_rejections=2, owned_input_rejections=4, public_fixture_runs=4,
                   named_selector_presence_checks=32, profiles=profiles)
    (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print('PASS: 4 feature builds; 64 root selector, 4 root bench, 4 private encode, 2 namespace and 4 owned-input rejections')


if __name__ == '__main__':
    main()
