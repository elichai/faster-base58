#!/usr/bin/env python3
"""Time public conversions of code_size.py probes and write a public timing CSV.

Pass LABEL=PATH for each `both`-direction cdylib (or Wasm module with --wasm).
LABEL is BACKEND or BACKEND+cache; no-alloc and bs58 stop at 64 KiB, five8 covers
32/64 B, and other names cover every size. --baseline prints paired ratios.
Each fresh worker process times every library, interleaved, in batches of at
least --min-ms. A cell reports the median of the process medians and their range.
Native workers use process CPU time; Wasm workers use V8 elapsed time.

--copies N times N fresh copies of each native library file instead of the file
itself, all interleaved: the same bytes can run at different speeds from different
files (page placement). A copy's value is its median over processes; a cell reports
the median over copies, their range (copy_min_ns, copy_max_ns), and the range of all
process medians. Cells whose copies differ by more than 5% are printed.
"""
import argparse
import csv
import ctypes
from datetime import date
import json
import os
from pathlib import Path
import random
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
SIZES = [0, 1, 8, 16, 21, 22, 23, 31, 32, 33, 63, 64, 65, 128, 256, 512, 1024, 2048, 4096,
         8192, 16384, 32768, 65536, 131072, 262144, 524288, 1048576, 2097152, 4194304]
WASM_FLAGS = ['--no-liftoff', '--no-wasm-lazy-compilation', '--no-wasm-async-compilation']


def supported(label, size):
    # Copies are labelled LABEL#INDEX.
    backend = label.partition('#')[0].split('+')[0]
    if backend == 'five8':
        return size in (32, 64)
    return size <= 65536 if backend in ('no-alloc', 'bs58') else True


def fixture(size):
    """Random bytes without leading zeros, fixed per size."""
    data = bytearray(random.Random(size).randbytes(size))
    if data:
        data[0] |= 1
    return bytes(data)


class Buffer:
    """`size` bytes starting at a 64-byte boundary, so every process times the same
    buffer alignment (ctypes only guarantees malloc's)."""

    def __init__(self, size, data=b''):
        self.raw = ctypes.create_string_buffer(size + 64)
        base = ctypes.addressof(self.raw)
        self.address, self.size = base + -base % 64, size
        ctypes.memmove(self.address, data, len(data))


def native_worker(job):
    signature = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t]
    functions = {}
    for label, path in job['libraries']:
        library = ctypes.CDLL(str(Path(path).resolve()))
        for operation, export in [('encode', 'size_repeat_encode'), ('decode', 'size_repeat_decode_prevalidated')]:
            function = getattr(library, export)
            function.argtypes, function.restype = signature, ctypes.c_size_t
            functions[label, operation] = function
    # A loader that reused one mapping for two files would time the same placement twice.
    assert len({ctypes.cast(f, ctypes.c_void_p).value for f in functions.values()}) == len(functions), \
        'two labels share one loaded library'

    def call(label, operation, source, length, output, count):
        start = time.process_time_ns()
        written = functions[label, operation](source.address, length, output.address, output.size, count)
        return time.process_time_ns() - start, written

    def result(label, operation, source, length, output):
        written = call(label, operation, source, length, output, 1)[1]
        assert written <= output.size, (label, operation, length)
        return ctypes.string_at(output.address, written)

    results = []
    for size in job['sizes']:
        data = Path(job['fixtures'], f'{size}.bin').read_bytes()
        labels = [label for label, _ in job['libraries'] if supported(label, size)]
        if not labels:
            continue
        encoded = Buffer(2 * size + 16)
        text = result(labels[0], 'encode', Buffer(size, data), size, encoded)
        # Decoding uses an exact buffer, which five8's fixed-width API requires.
        for operation, value, expected, output in [('encode', data, text, encoded),
                                                    ('decode', text, data, Buffer(size))]:
            source = Buffer(len(value), value)
            for label in labels:
                assert result(label, operation, source, len(value), output) == expected, (label, operation, size)
            results += time_cells(job, labels, operation, size,
                                  lambda label, count: call(label, operation, source, len(value), output, count)[0])
    return results


def time_cells(job, labels, operation, size, batch):
    counts = {}
    for label in labels:
        count = 1
        while batch(label, count) < job['min_ns']:
            count *= 2
        counts[label] = count
    samples = {label: [] for label in labels}
    for round_ in range(job['batches']):
        rotation = round_ % len(labels)
        for label in labels[rotation:] + labels[:rotation]:
            samples[label].append(batch(label, counts[label]) / counts[label])
    return [dict(label=label, operation=operation, size=size, ns=statistics.median(times),
                 iterations=counts[label]) for label, times in samples.items()]


def summarize(processes):
    """Per (label, operation, size), in first-process order: (median over copies of each
    copy's process median, sorted copy medians, sorted process medians of every copy,
    first-process iterations). A label without copies is one copy."""
    cells = {}
    for process in processes:
        for cell in process:
            label, _, copy = cell['label'].partition('#')
            entry = cells.setdefault((label, cell['operation'], cell['size']), ({}, cell['iterations']))
            entry[0].setdefault(copy, []).append(cell['ns'])
    result = []
    for key, (copies, iterations) in cells.items():
        medians = sorted(statistics.median(times) for times in copies.values())
        result.append((key, statistics.median(medians), medians,
                       sorted(ns for times in copies.values() for ns in times), iterations))
    return result


def per_process(processes):
    """Each process's times keyed by (label, operation, size), copies merged by median."""
    result = []
    for process in processes:
        grouped = {}
        for cell in process:
            grouped.setdefault((cell['label'].partition('#')[0], cell['operation'], cell['size']), []).append(cell['ns'])
        result.append({key: statistics.median(times) for key, times in grouped.items()})
    return result


def self_test():
    for size in (0, 1, 63, 4096):
        buffer = Buffer(size, b'x' * size)
        assert buffer.address % 64 == 0 and ctypes.string_at(buffer.address, size) == b'x' * size
    assert supported('bs58#3', 65536) and not supported('bs58#3', 131072)
    assert supported('five8#0', 32) and not supported('five8#0', 33) and supported('auto+cache#2', 1 << 22)
    cell = lambda label, ns: dict(label=label, operation='decode', size=64, ns=ns, iterations=8)
    processes = [[cell('auto#0', 10), cell('auto#1', 20), cell('auto#2', 30), cell('bs58', 5)],
                 [cell('auto#0', 12), cell('auto#1', 22), cell('auto#2', 31), cell('bs58', 7)],
                 [cell('auto#0', 11), cell('auto#1', 26), cell('auto#2', 40), cell('bs58', 6)]]
    assert summarize(processes) == [(('auto', 'decode', 64), 22, [11, 22, 31], [10, 11, 12, 20, 22, 26, 30, 31, 40], 8),
                                    (('bs58', 'decode', 64), 6, [6], [5, 6, 7], 8)], summarize(processes)
    assert per_process(processes)[2] == {('auto', 'decode', 64): 26, ('bs58', 'decode', 64): 6}
    print('public_bench self-test passed')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('libraries', nargs='*', metavar='LABEL=PATH')
    parser.add_argument('--output', type=Path, help='new CSV file')
    parser.add_argument('--platform', help='figure/table platform key, such as m5, gcp or wasm')
    parser.add_argument('--source-commit', help='library commit under test (default: this checkout)')
    parser.add_argument('--wasm', action='store_true', help='time Wasm modules with Node/V8')
    parser.add_argument('--processes', type=int, default=3)
    parser.add_argument('--batches', type=int, default=3)
    parser.add_argument('--min-ms', type=float, default=10)
    parser.add_argument('--sizes', type=lambda text: [int(n) for n in text.split(',')], default=SIZES)
    parser.add_argument('--cpu', type=int, help='pin workers to this CPU (Linux)')
    parser.add_argument('--baseline', help='print each cell relative to this label')
    parser.add_argument('--copies', type=int, default=1, help='time N fresh copies of each native library')
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--worker', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.worker:
        print(json.dumps(native_worker(json.loads(args.worker))))
        return
    if not (args.libraries and args.output and args.platform):
        parser.error('LABEL=PATH, --output and --platform are required')
    if args.copies < 1 or (args.copies > 1 and args.wasm):
        parser.error('--copies takes a positive count and applies to native libraries')
    libraries = [spec.split('=', 1) for spec in args.libraries]
    for label, path in libraries:
        backend, _, policy = label.partition('+')
        if not backend or '#' in label or policy not in ('', 'cache') or not Path(path).is_file():
            parser.error(f'bad library: {label}={path}')
    if args.baseline and args.baseline not in dict(libraries):
        parser.error(f'unknown baseline: {args.baseline}')
    if not any(supported(label, size) for label, _ in libraries for size in args.sizes):
        parser.error('no library supports any requested size')
    if args.output.exists():
        parser.error(f'{args.output} exists')
    if args.cpu is not None:
        os.sched_setaffinity(0, {args.cpu})
    commit = args.source_commit or subprocess.run(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'],
                                                  capture_output=True, text=True).stdout.strip()
    if not commit:
        parser.error('--source-commit is required outside a Git checkout')
    processes = []
    with tempfile.TemporaryDirectory(prefix='base58-public-') as fixtures:
        for size in args.sizes:
            Path(fixtures, f'{size}.bin').write_bytes(fixture(size))
        timed = libraries
        if args.copies > 1:
            timed = []
            for label, path in libraries:
                for copy in range(args.copies):
                    name = f'{label}#{copy}'
                    timed.append((name, str(shutil.copyfile(path, Path(fixtures, name + Path(path).suffix)))))
        for index in range(args.processes):
            # Rotate the load order so no library always loads first.
            order = timed[index % len(timed):] + timed[:index % len(timed)]
            job = json.dumps(dict(libraries=order, fixtures=fixtures, sizes=args.sizes,
                                  batches=args.batches, min_ns=int(args.min_ms * 1e6)))
            command = (['node', *WASM_FLAGS, str(ROOT / 'scripts/public_bench_wasm.mjs'), job] if args.wasm
                       else [sys.executable, '-B', __file__, '--worker', job])
            processes.append(json.loads(subprocess.run(command, check=True, stdout=subprocess.PIPE, text=True).stdout))
            print(f'process {index + 1}/{args.processes} done', file=sys.stderr, flush=True)
    rows = []
    for (label, operation, size), warm, copies, medians, iterations in summarize(processes):
        backend, _, policy = label.partition('+')
        rows.append(dict(platform=args.platform, cache=policy or 'default', backend=backend,
                         operation=operation, decoded_bytes=size,
                         warm_ns=f'{warm:.4g}', process_min_ns=f'{medians[0]:.4g}',
                         process_max_ns=f'{medians[-1]:.4g}', iterations=iterations,
                         clock='elapsed' if args.wasm else 'process CPU', processes=args.processes,
                         batches=args.batches, measurement_date=date.today().isoformat(), source_commit=commit,
                         copies=len(copies), copy_min_ns=f'{copies[0]:.4g}', copy_max_ns=f'{copies[-1]:.4g}'))
        if copies[-1] > 1.05 * copies[0]:
            print(f'copy spread {copies[-1] / copies[0]:.3f}: {label} {operation} {size}')
    with args.output.open('x', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    if args.baseline:
        # Each label against the baseline with the same cache policy, paired within
        # processes: the median and range of the per-process ratios.
        times = per_process(processes)
        for label, operation, size in times[0]:
            policy = label.partition('+')[2]
            base = args.baseline.partition('+')[0] + ('+' + policy if policy else '')
            if label == base or (base, operation, size) not in times[0]:
                continue
            ratios = sorted(t[label, operation, size] / t[base, operation, size] for t in times)
            print(f'{label:>16} {operation:6} {size:8} {statistics.median(ratios):.3f} '
                  f'[{ratios[0]:.3f}, {ratios[-1]:.3f}]')
    print(args.output)


if __name__ == '__main__':
    main()
