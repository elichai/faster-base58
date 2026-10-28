#!/usr/bin/env python3
"""Run existing guarded Rust tests with rebuilt std and fatal sanitizer failures."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

from process_supervisor import install_signal_handlers, owned_process

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
    listed = 'first: test\nscalar::nested: test\n\n2 tests, 0 benchmarks\n'
    assert parse_discovery(listed) == ['first', 'scalar::nested']
    for bad in ['', '0 tests, 0 benchmarks', listed + listed,
                listed.replace('2 tests', '3 tests'), listed.replace('0 benchmarks', '1 benchmark')]:
        try:
            parse_discovery(bad)
        except ValueError:
            pass
        else:
            raise AssertionError('empty/ambiguous/mismatched discovery accepted')
    print('run_native_sanitizer self-test: pass')


def parse_result(text, expected):
    results = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', text)
    if expected <= 0 or len(results) != 1 or tuple(map(int, results[0])) != (expected, 0, 0):
        raise ValueError('expected exactly ' + str(expected) + ' passed, 0 failed, 0 ignored')
    skipped = 'skipped: no AVX-512 IFMA' in text
    return {'passed': expected, 'covered': 0 if skipped else expected, 'isa_skipped': skipped}


def parse_discovery(text):
    names = re.findall(r'^(.+): test$', text, re.MULTILINE)
    summaries = re.findall(r'^(\d+) tests?, (\d+) benchmarks?$', text, re.MULTILINE)
    if (not names or len(names) != len(set(names)) or len(summaries) != 1
            or tuple(map(int, summaries[0])) != (len(names), 0)):
        raise ValueError('expected a nonempty, unique test list and one matching zero-benchmark summary')
    return names


def main():
    install_signal_handlers()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('sanitizer', nargs='?', choices=['memory', 'thread'])
    parser.add_argument('--log-dir', type=Path)
    parser.add_argument('--target-dir', type=Path, help='reuse a retained instrumented build cache')
    parser.add_argument('--target', choices=[TARGET, 'aarch64-apple-darwin'], default=TARGET)
    parser.add_argument('--toolchain', default=TOOLCHAIN)
    parser.add_argument('--cache', choices=['off', 'on'], default='on')
    parser.add_argument('--compact', action='store_true')
    parser.add_argument('--require-ifma', action='store_true')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if args.sanitizer is None:
        parser.error('select memory or thread')
    if args.sanitizer == 'memory' and args.target != TARGET:
        parser.error('the memory lane requires x86_64 Linux')
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
               CARGO_TARGET_DIR=str(args.target_dir or logs.parent / 'build'),
               MSAN_OPTIONS='halt_on_error=1:exit_code=86', TSAN_OPTIONS='halt_on_error=1')
    env.setdefault('CARGO_BUILD_JOBS', '1')
    env.setdefault('PROPTEST_CASES', '512')
    if args.require_ifma:
        env['BASE58_REQUIRE_IFMA'] = '1'
    version = subprocess.check_output(['rustup', 'run', args.toolchain, 'rustc', '-vV'], env=env, text=True)
    (logs / 'tool.json').write_text(json.dumps({'rustc': version, 'rustflags': flags,
        'git_revision': env.get('GITHUB_SHA'), 'require_ifma': args.require_ifma,
        'target': args.target, 'cache': args.cache, 'compact': args.compact,
        'proptest_cases': env['PROPTEST_CASES']}, indent=2) + '\n')
    features = '__internals,cache' if args.cache == 'on' else '__internals'
    if args.compact:
        features += ',compact'
    base = ['cargo', '+' + args.toolchain, 'test', '--locked', '-Zbuild-std', '--target', args.target,
            '--no-default-features', '--features', features]
    tests = MEMORY_TESTS if args.sanitizer == 'memory' else THREAD_TESTS
    cases = [('startup-list', ['--lib', '--', '--list'], None, {})]
    for name in tests:
        extra = {'BASE58_NTT_CACHE_HISTORY': 'concurrent'} if name == THREAD_TESTS[0] else {}
        cases.append((name, ['--lib', name, '--', '--exact', '--nocapture', '--test-threads=1'], 1, extra))
    if args.sanitizer == 'memory':
        for label, target in [('public-api', 'api'), ('core-vectors', 'bitcoin_core'),
                              ('owned-output', 'owned_output')]:
            cases.append((label + '-list', ['--test', target, '--', '--list'], ('discover', target), {}))
            cases.append((label, ['--test', target, '--', '--nocapture', '--test-threads=1'],
                          ('discovered', target), {}))
    else:
        cases.append(('matrix-cold', ['--test', 'matrix_cold', '--', '--nocapture', '--test-threads=1'], 1, {}))
    covered, skipped = 0, []
    discovered = {}
    for name, options, expected, extra in cases:
        print(name, flush=True)
        command = base + options
        path = logs / (name.replace('::', '-') + '.log')
        start = time.monotonic()
        with path.open('x') as output:
            with owned_process(command, cwd=repository, env=dict(env, **extra),
                               stdout=output, stderr=subprocess.STDOUT) as child:
                try:
                    code = child.wait(timeout=1800)
                except subprocess.TimeoutExpired:
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
                names = parse_discovery(text)
                row['discovered_tests'] = names
                if not all(name in names for name in tests):
                    raise ValueError('test-harness discovery omitted a selected test')
            elif isinstance(expected, tuple) and expected[0] == 'discover':
                names = parse_discovery(text)
                discovered[expected[1]] = names
                row['discovered_tests'] = names
            else:
                count = len(discovered[expected[1]]) if isinstance(expected, tuple) else expected
                result = parse_result(text, count)
                if args.require_ifma and result['isa_skipped']:
                    raise ValueError('required IFMA execution was skipped')
                row.update(result)
                covered += result['covered']
                if result['isa_skipped']:
                    skipped.append(name)
        except ValueError as error:
            row['validation_error'] = str(error)
        with (logs / 'commands.jsonl').open('a') as output:
            output.write(json.dumps(row) + '\n')
        if 'validation_error' in row:
            failure = {'status': 'failed', 'failed_command': name,
                       'validation_error': row['validation_error'],
                       'covered_tests': covered, 'isa_skipped': skipped}
            (logs / 'complete.json').write_text(json.dumps(failure, indent=2) + '\n')
            raise SystemExit(row['validation_error'])
    summary = {'status': 'pass_with_isa_skips' if skipped else 'pass',
               'test_commands': sum(expected is not None and not (isinstance(expected, tuple)
                                    and expected[0] == 'discover') for _, _, expected, _ in cases),
               'discovered_integration_tests': discovered,
               'covered_tests': covered, 'isa_skipped': skipped}
    (logs / 'complete.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary), flush=True)


if __name__ == '__main__':
    main()
