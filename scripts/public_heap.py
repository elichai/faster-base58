#!/usr/bin/env python3
"""Count public borrowed conversions, independently cold per operation/input size.

Build empty,auto,no-alloc cdylibs with code_size.py --count-allocations, then pass
--build DIRECTORY --output NEW_DIRECTORY. Native jobs use fresh processes;
Wasm jobs use fresh Node processes/instances through wasm_allocations.mjs.
Only requested Rust layouts are counted. This is not a timing or RSS probe.
"""
import argparse
import csv
import ctypes
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import platform
import subprocess
import sys

from public_bench import SIZES

ROOT = Path(__file__).resolve().parents[1]
METRICS = ['alloc_calls', 'alloc_bytes', 'zeroed_calls', 'zeroed_bytes', 'realloc_calls',
           'realloc_old_bytes', 'realloc_new_bytes', 'dealloc_calls', 'dealloc_bytes',
           'failed_calls', 'live_before_bytes', 'peak_live_bytes', 'live_after_bytes']


def fixture(size):
    """Independent integer oracle: the Base58 text consists entirely of '2'."""
    if size == 0:
        return b'', b''
    digits = math.ceil((8 * (size - 1) + math.log2(57)) / math.log2(58))
    value = (58 ** digits - 1) // 57
    assert (value.bit_length() + 7) // 8 == size
    return value.to_bytes(size, 'big'), b'2' * digits


def validate(stats):
    assert stats['failed_calls'] == 0
    assert stats['alloc_bytes'] + stats['realloc_new_bytes'] - stats['realloc_old_bytes'] - stats['dealloc_bytes'] == \
        stats['live_after_bytes'] - stats['live_before_bytes']
    assert stats['peak_live_bytes'] >= max(stats['live_before_bytes'], stats['live_after_bytes'])
    assert stats['zeroed_calls'] <= stats['alloc_calls']
    stats['peak_extra_bytes'] = stats['peak_live_bytes'] - stats['live_before_bytes']
    stats['retained_delta_bytes'] = stats['live_after_bytes'] - stats['live_before_bytes']
    stats['requests'] = stats['alloc_calls'] + stats['realloc_calls']


def native_worker(job):
    api = ctypes.CDLL(job['artifact'])
    api.size_alloc.argtypes, api.size_alloc.restype = [ctypes.c_size_t], ctypes.c_void_p
    api.size_dealloc.argtypes, api.size_dealloc.restype = [ctypes.c_void_p, ctypes.c_size_t], None
    for name in ['size_count_self_check', 'size_count_begin', 'size_count_end']:
        function = getattr(api, name)
        function.argtypes, function.restype = [], None
    api.size_count_metrics.argtypes, api.size_count_metrics.restype = [], ctypes.c_size_t
    api.size_count_get.argtypes, api.size_count_get.restype = [ctypes.c_size_t], ctypes.c_size_t
    assert api.size_count_metrics() == len(METRICS)
    api.size_count_self_check()
    function = getattr(api, 'size_' + job['operation'])
    function.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t]
    function.restype = ctypes.c_size_t
    data, text = [Path(job[key]).read_bytes() for key in ['data', 'text']]
    value, expected = (data, text) if job['operation'] == 'encode' else (text, data)
    capacity = max(len(value), len(expected)) if job['backend'] == 'empty' else len(expected)
    source_length, output_length = len(value) + 34, capacity + 38
    source, output = api.size_alloc(source_length), api.size_alloc(output_length)
    assert source and output
    assert source + source_length <= output or output + output_length <= source
    ctypes.memset(source, 0xA5, source_length)
    ctypes.memmove(source + 17, value, len(value))
    rows = []
    try:
        for phase in ['cold', 'warm']:
            ctypes.memset(output, 0xA5, output_length)
            api.size_count_begin()
            written = function(source + 17, len(value), output + 19, capacity)
            api.size_count_end()
            stats = {name: api.size_count_get(index) for index, name in enumerate(METRICS)}
            validate(stats)
            assert ctypes.string_at(source, source_length) == b'\xA5' * 17 + value + b'\xA5' * 17
            assert written == (len(value) if job['backend'] == 'empty' else len(expected))
            if job['backend'] != 'empty':
                assert ctypes.string_at(output + 19, written) == expected
            assert ctypes.string_at(output, 19) == b'\xA5' * 19
            assert ctypes.string_at(output + 19 + written, output_length - 19 - written) == b'\xA5' * (output_length - 19 - written)
            rows.append(dict(phase=phase, statistics=stats,
                             input_allocation_bytes=source_length, output_allocation_bytes=output_length))
    finally:
        api.size_dealloc(output, output_length)
        api.size_dealloc(source, source_length)
    return rows


def self_test():
    alphabet = b'123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
    for size in [0, 1, 2, 8, 32, 64, 512, 4096]:
        data, text = fixture(size)
        value = 0
        for digit in text:
            value = value * 58 + alphabet.index(digit)
        assert value == int.from_bytes(data, 'big') and len(data) == size
    stats = dict(zip(METRICS, [2, 24, 1, 8, 1, 16, 32, 2, 40, 0, 5, 45, 5]))
    validate(stats)
    assert stats['requests'] == 3 and stats['peak_extra_bytes'] == 40 and stats['retained_delta_bytes'] == 0
    try:
        validate(dict(stats, dealloc_bytes=39))
    except AssertionError:
        pass
    else:
        raise AssertionError('invalid allocation balance accepted')
    print('public_heap self-test passed')


def main():
    if sys.flags.optimize:
        raise SystemExit('public_heap requires Python assertions; omit -O and unset PYTHONOPTIMIZE')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--sizes', type=lambda value: [int(n) for n in value.split(',')], default=SIZES)
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--worker', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.worker:
        print(json.dumps(native_worker(json.loads(args.worker))))
        return
    if not args.build or not args.output:
        parser.error('--build and --output are required')
    assert args.repeats > 0 and len(set(args.sizes)) == len(args.sizes)
    assert all(0 <= size <= 4194304 for size in args.sizes)
    build = args.build.resolve()
    metadata = json.loads((build / 'metadata.json').read_text())
    assert metadata['count_allocations'], 'requires instrumented artifacts'
    with (build / 'sizes.csv').open() as stream:
        artifacts = list(csv.DictReader(stream))
    assert all(row['backend'] in ['empty', 'auto', 'no-alloc'] and row['kind'] == 'cdylib'
               and row['direction'] == 'both' for row in artifacts)
    assert len({row['backend'] for row in artifacts}) == len(artifacts) and artifacts
    wasm = metadata['target'] == 'wasm32-unknown-unknown'
    if not wasm:
        host = subprocess.check_output(['rustc', '-vV'], text=True).split('host: ')[1].splitlines()[0]
        assert host == metadata['target'], 'native target must match this host'
    args.output.mkdir(parents=True, exist_ok=False)
    output = args.output.resolve()
    for size in args.sizes:
        data, text = fixture(size)
        (output / f'{size}.bin').write_bytes(data)
        (output / f'{size}.txt').write_bytes(text)
    record = dict(measured_utc=datetime.now(timezone.utc).isoformat(), build=metadata,
                  host=platform.platform(), machine=platform.machine(), repeats=args.repeats,
                  oracle='integer geometric series: (58**digits - 1)//57; text is all 2',
                  clock='none; untimed allocation windows', sizes=args.sizes, status='running',
                  harness_sha256={name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
                                  for name in ['scripts/public_heap.py', 'scripts/public_bench.py', 'scripts/wasm_allocations.mjs']})
    (output / 'metadata.json').write_text(json.dumps(record, indent=2))
    rows = []
    with (output / 'raw.jsonl').open('x') as raw:
        for artifact in artifacts:
            extension = '.wasm' if wasm else '.dylib' if 'apple' in metadata['target'] else '.so'
            path = build / (artifact['backend'] + '-both-cdylib' + extension + '.stripped')
            assert hashlib.sha256(path.read_bytes()).hexdigest() == artifact['stripped_sha256']
            for size in args.sizes:
                if artifact['backend'] in ['empty', 'no-alloc'] and size > 65536:
                    continue
                for operation in ['encode', 'decode']:
                    for repeat in range(args.repeats):
                        job = dict(artifact=str(path), backend=artifact['backend'], operation=operation,
                                   data=str(output / f'{size}.bin'), text=str(output / f'{size}.txt'))
                        command = ['node', str(ROOT / 'scripts/wasm_allocations.mjs'), '--worker'] if wasm else \
                            [sys.executable, '-B', str(Path(__file__).resolve()), '--worker']
                        events = json.loads(subprocess.check_output(command + [json.dumps(job)], text=True, timeout=120))
                        for event in events:
                            stats = event.pop('statistics')
                            validate(stats)
                            io_bytes = event['input_allocation_bytes'] + event['output_allocation_bytes']
                            if event['phase'] == 'cold':
                                assert stats['live_before_bytes'] == io_bytes
                            if artifact['backend'] != 'auto':
                                assert all(stats[name] == 0 for name in METRICS[:10])
                            if not metadata['cache'] or event['phase'] == 'warm':
                                assert stats['retained_delta_bytes'] == 0
                            row = dict(backend=artifact['backend'], operation=operation, decoded_bytes=size,
                                       repeat=repeat, **event, **stats,
                                       peak_internal_bytes=stats['peak_live_bytes'] - io_bytes,
                                       retained_internal_bytes=stats['live_after_bytes'] - io_bytes)
                            raw.write(json.dumps(row) + '\n')
                            raw.flush()
                            rows.append(row)
            print(f"Counted {artifact['backend']}: {metadata['target']}, cache={metadata['cache']}", flush=True)
    with (output / 'heap.csv').open('x', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(dict.fromkeys(key for row in rows for key in row)))
        writer.writeheader()
        writer.writerows(rows)
    record['status'], record['rows'] = 'complete', len(rows)
    (output / 'metadata.json').write_text(json.dumps(record, indent=2))


if __name__ == '__main__':
    main()
