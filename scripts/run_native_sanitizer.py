#!/usr/bin/env python3
"""Run existing guarded Rust tests with rebuilt std and fatal sanitizer failures."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time

TOOLCHAIN = 'nightly-2026-10-26'
TARGET = 'x86_64-unknown-linux-gnu'
MEMORY_TESTS = [
    'matrix::tests::best_matches_portable',
    'encode::matrix::tests::best_matches_portable',
    'ntt::tests::small_shapes_best',
    'ntt::tests::scratch_at_any_alignment',
    'ntt::tests::prepared_powers_match_karatsuba',
    'mul_ifma::tests::avx512_matches_reference',
]
THREAD_TESTS = [
    'ntt::cache_policy_tests::retention_is_bounded_across_histories',
    'ntt::tests::cached_tables_across_threads',
    'matrix::tests::threads',
    'encode::matrix::tests::threads',
]


def self_test():
    good = 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out;'
    assert parse_result(good, 1)['covered'] == 1, 'valid result not counted'
    for bad in [good.replace('1 passed', '0 passed'), good.replace('0 failed', '1 failed'),
                good.replace('0 ignored', '1 ignored'), good + '\n' + good, '']:
        try:
            parse_result(bad, 1)
        except ValueError:
            pass
        else:
            raise AssertionError('zero/failed/ignored/ambiguous result accepted')
    skipped = parse_result('skipped: no AVX-512 IFMA\n' + good, 1)
    assert skipped['covered'] == 0 and skipped['isa_skipped']
    assert parse_result(good.replace('1 passed', '3 passed'), 3)['covered'] == 3
    print('run_native_sanitizer self-test: pass')


def parse_result(text, expected):
    results = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', text)
    if expected <= 0 or len(results) != 1 or tuple(map(int, results[0])) != (expected, 0, 0):
        raise ValueError('expected exactly ' + str(expected) + ' passed, 0 failed, 0 ignored')
    skipped = 'skipped: no AVX-512 IFMA' in text
    return {'passed': expected, 'covered': 0 if skipped else expected, 'isa_skipped': skipped}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('sanitizer', nargs='?', choices=['memory', 'thread'])
    parser.add_argument('--log-dir', type=Path)
    parser.add_argument('--require-ifma', action='store_true')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if args.sanitizer is None:
        parser.error('select memory or thread')
    repository = Path(__file__).resolve().parents[1]
    logs = args.log_dir or repository / 'target/native-sanitizers' / args.sanitizer / 'logs'
    logs.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ)
    for key in ['CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTDOC']:
        env.pop(key, None)
    flags = ('-Zsanitizer=' + args.sanitizer +
             (' -Zsanitizer-memory-track-origins=2' if args.sanitizer == 'memory' else '') +
             ' -Zextra-const-ub-checks -Zstrict-init-checks --cfg careful'
             ' -Cdebug-assertions=yes -Coverflow-checks=yes')
    env.update(RUSTFLAGS=flags, CARGO_INCREMENTAL='0',
               CARGO_TARGET_DIR=str(logs.parent / 'build'),
               MSAN_OPTIONS='halt_on_error=1:exit_code=86', TSAN_OPTIONS='halt_on_error=1')
    env.setdefault('CARGO_BUILD_JOBS', '1')
    if args.require_ifma:
        env['BASE58_REQUIRE_IFMA'] = '1'
    version = subprocess.check_output(['rustup', 'run', TOOLCHAIN, 'rustc', '-vV'], env=env, text=True)
    (logs / 'tool.json').write_text(json.dumps({'rustc': version, 'rustflags': flags,
        'git_revision': env.get('GITHUB_SHA'), 'require_ifma': args.require_ifma}, indent=2) + '\n')
    base = ['cargo', '+' + TOOLCHAIN, 'test', '--locked', '-Zbuild-std', '--target', TARGET,
            '--no-default-features', '--features', '__internals,cache']
    tests = MEMORY_TESTS if args.sanitizer == 'memory' else THREAD_TESTS
    cases = [('startup-list', ['--lib', '--', '--list'], None, {})]
    for name in tests:
        extra = {'BASE58_NTT_CACHE_HISTORY': 'concurrent'} if name == THREAD_TESTS[0] else {}
        cases.append((name, ['--lib', name, '--', '--exact', '--nocapture', '--test-threads=1'], 1, extra))
    if args.sanitizer == 'memory':
        cases.append(('public-api', ['--test', 'api', '--', '--nocapture', '--test-threads=1'], 3, {}))
    else:
        cases.append(('matrix-cold', ['--test', 'matrix_cold', '--', '--nocapture', '--test-threads=1'], 1, {}))
    covered, skipped = 0, []
    for name, options, expected, extra in cases:
        print(name, flush=True)
        command = base + options
        path = logs / (name.replace('::', '-') + '.log')
        start = time.monotonic()
        with path.open('x') as output:
            child = subprocess.Popen(command, cwd=repository, env=dict(env, **extra),
                                     stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                code = child.wait(timeout=1800)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
                code = 'timeout'
        text = path.read_text()
        print(text, flush=True)
        row = {'name': name, 'command': command, 'exit_code': code,
               'seconds': time.monotonic() - start,
               'log_sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'environment': extra}
        try:
            if code != 0:
                raise ValueError('test process failed: ' + str(code))
            if expected is None:
                if not all(name + ': test' in text for name in tests):
                    raise ValueError('test-harness discovery omitted a selected test')
            else:
                result = parse_result(text, expected)
                row.update(result)
                covered += result['covered']
                if result['isa_skipped']:
                    skipped.append(name)
        except ValueError as error:
            row['validation_error'] = str(error)
        with (logs / 'commands.jsonl').open('a') as output:
            output.write(json.dumps(row) + '\n')
        if 'validation_error' in row:
            raise SystemExit(row['validation_error'])
    summary = {'status': 'pass', 'test_commands': len(cases) - 1,
               'covered_tests': covered, 'isa_skipped': skipped}
    (logs / 'complete.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary), flush=True)


if __name__ == '__main__':
    main()
