"""Repeat unchanged controls and formatter inputs in fresh reversed-load processes."""
import ctypes
import json
import os
from pathlib import Path
import random
import statistics
import subprocess
import sys
import time

root = Path('/dev/shm/base58-pr1-improve')
if len(sys.argv) == 1:
    rows = []
    for cpu in ('generic', 'native'):
        for backend in ('auto', 'no-alloc'):
            for order in ('normal', 'reverse'):
                for run in range(3):
                    row = json.loads(subprocess.check_output([sys.executable, __file__, cpu, backend, order], text=True))
                    row.update(cpu=cpu, backend=backend, order=order, run=run)
                    rows.append(row)
    destination = root / 'top-fresh-controls.json'
    assert not destination.exists()
    destination.write_text(json.dumps(rows, indent=2) + '\n')
    for cpu in ('generic', 'native'):
        for backend in ('auto', 'no-alloc'):
            for case in ('encode-zero-512', 'decode-random-22', 'encode-random-16', 'encode-random-31'):
                subset = [r for r in rows if r['cpu'] == cpu and r['backend'] == backend]
                ratios = [r['cases'][case][1]['median_ns'] / r['cases'][case][0]['median_ns'] for r in subset]
                print(cpu, backend, case, 'candidate/reference', f'{statistics.median(ratios):.3f}',
                      'range', f'{min(ratios):.3f}..{max(ratios):.3f}', flush=True)
    sys.exit()

cpu, backend, order = sys.argv[1:]
os.sched_setaffinity(0, {min(os.sched_getaffinity(0))})
paths = [root / ('base58-pr1-reference-' + cpu) / (backend + '-both-cdylib.so'),
         root / ('base58-pr1-ten-top-' + cpu) / (backend + '-both-cdylib.so')]
libraries = {}
for variant in ((0, 1) if order == 'normal' else (1, 0)):
    libraries[variant] = ctypes.CDLL(str(paths[variant]))
result = {'addresses': [], 'cases': {}}
for variant in range(2):
    result['addresses'].append(ctypes.cast(libraries[variant].size_repeat_encode, ctypes.c_void_p).value)
for case in ('encode-zero-512', 'decode-random-22', 'encode-random-16', 'encode-random-31'):
    operation, pattern, size_text = case.split('-')
    size = int(size_text)
    data = bytes(size) if pattern == 'zero' else random.Random(size).randbytes(size)
    source = ctypes.create_string_buffer(data)
    output = ctypes.create_string_buffer(size * 2 + 16)
    if operation == 'decode':
        encode = libraries[0].size_encode
        encode.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t]
        encode.restype = ctypes.c_size_t
        length = encode(source, size, output, len(output))
        source = ctypes.create_string_buffer(output.raw[:length])
        size = length
    functions = [getattr(libraries[v], 'size_repeat_encode' if operation == 'encode' else 'size_repeat_decode_prevalidated') for v in range(2)]
    for function in functions:
        function.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t]
        function.restype = ctypes.c_size_t
    def batch(variant, iterations):
        start = time.process_time_ns()
        length = functions[variant](source, size, output, len(output), iterations)
        elapsed = time.process_time_ns() - start
        assert length <= len(output)
        return elapsed, length
    count = 1
    while min(batch(v, count)[0] for v in range(2)) < 10_000_000:
        count *= 2
    samples = [[], []]
    lengths = []
    for sample in range(9):
        for variant in ((0, 1) if sample % 2 == 0 else (1, 0)):
            elapsed, length = batch(variant, count)
            samples[variant].append(elapsed / count)
            lengths.append(length)
    assert len(set(lengths)) == 1
    result['cases'][case] = [{'median_ns': statistics.median(s), 'samples_ns': s, 'iterations': count} for s in samples]
print(json.dumps(result))
