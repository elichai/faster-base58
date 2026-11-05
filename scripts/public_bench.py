#!/usr/bin/env python3
"""Time public conversions of code_size.py probes and write the README timing CSV.

Pass LABEL=PATH for each `both`-direction cdylib (or Wasm module with --wasm).
LABEL is BACKEND or BACKEND+cache; no-alloc and bs58 stop at 64 KiB, five8 covers
32/64 B, and other names cover every size. --baseline prints paired ratios.
Each fresh worker process times every library, interleaved, in batches of at
least --min-ms. A cell reports the median of the process medians and their range.
Native workers use process CPU time; Wasm workers use V8 elapsed time.
"""
import argparse
import csv
import ctypes
from datetime import date
import json
import os
from pathlib import Path
import random
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
    backend = label.split('+')[0]
    if backend == 'five8':
        return size in (32, 64)
    return size <= 65536 if backend in ('no-alloc', 'bs58') else True


def fixture(size):
    """Random bytes without leading zeros, fixed per size."""
    data = bytearray(random.Random(size).randbytes(size))
    if data:
        data[0] |= 1
    return bytes(data)


def native_worker(job):
    signature = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t]
    functions = {}
    for label, path in job['libraries']:
        library = ctypes.CDLL(str(Path(path).resolve()))
        for operation, export in [('encode', 'size_repeat_encode'), ('decode', 'size_repeat_decode_prevalidated')]:
            function = getattr(library, export)
            function.argtypes, function.restype = signature, ctypes.c_size_t
            functions[label, operation] = function

    def call(label, operation, source, length, output, count):
        start = time.process_time_ns()
        written = functions[label, operation](source, length, output, len(output), count)
        return time.process_time_ns() - start, written

    def result(label, operation, source, length, output):
        written = call(label, operation, source, length, output, 1)[1]
        assert written <= len(output), (label, operation, length)
        return ctypes.string_at(output, written)

    results = []
    for size in job['sizes']:
        data = Path(job['fixtures'], f'{size}.bin').read_bytes()
        labels = [label for label, _ in job['libraries'] if supported(label, size)]
        if not labels:
            continue
        encoded = ctypes.create_string_buffer(2 * size + 16)
        text = result(labels[0], 'encode', ctypes.create_string_buffer(data, size), size, encoded)
        # Decoding uses an exact buffer, which five8's fixed-width API requires.
        for operation, value, expected, output in [('encode', data, text, encoded),
                                                    ('decode', text, data, ctypes.create_string_buffer(size))]:
            source = ctypes.create_string_buffer(value, len(value))
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
    parser.add_argument('--worker', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.worker:
        print(json.dumps(native_worker(json.loads(args.worker))))
        return
    if not (args.libraries and args.output and args.platform):
        parser.error('LABEL=PATH, --output and --platform are required')
    libraries = [spec.split('=', 1) for spec in args.libraries]
    for label, path in libraries:
        backend, _, policy = label.partition('+')
        if not backend or policy not in ('', 'cache') or not Path(path).is_file():
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
        for index in range(args.processes):
            # Rotate the load order so no library always loads first.
            order = libraries[index % len(libraries):] + libraries[:index % len(libraries)]
            job = json.dumps(dict(libraries=order, fixtures=fixtures, sizes=args.sizes,
                                  batches=args.batches, min_ns=int(args.min_ms * 1e6)))
            command = (['node', *WASM_FLAGS, str(ROOT / 'scripts/public_bench_wasm.mjs'), job] if args.wasm
                       else [sys.executable, '-B', __file__, '--worker', job])
            processes.append(json.loads(subprocess.run(command, check=True, stdout=subprocess.PIPE, text=True).stdout))
            print(f'process {index + 1}/{args.processes} done', file=sys.stderr, flush=True)
    rows = []
    for cell in processes[0]:
        key = (cell['label'], cell['operation'], cell['size'])
        medians = sorted(next(c['ns'] for c in process if (c['label'], c['operation'], c['size']) == key)
                         for process in processes)
        backend, _, policy = cell['label'].partition('+')
        rows.append(dict(platform=args.platform, cache=policy or 'default', backend=backend,
                         operation=cell['operation'], decoded_bytes=cell['size'],
                         warm_ns=f'{statistics.median(medians):.4g}', process_min_ns=f'{medians[0]:.4g}',
                         process_max_ns=f'{medians[-1]:.4g}', iterations=cell['iterations'],
                         clock='elapsed' if args.wasm else 'process CPU', processes=args.processes,
                         batches=args.batches, measurement_date=date.today().isoformat(), source_commit=commit))
    with args.output.open('x', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    if args.baseline:
        # Each label against the baseline with the same cache policy, paired within
        # processes: the median and range of the per-process ratios.
        times = [{(c['label'], c['operation'], c['size']): c['ns'] for c in process} for process in processes]
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
