"""Compare normal public APIs from two shared libraries in one process."""
import ctypes
import json
import pathlib
import random
import statistics
import sys
import time

old, new, destination = sys.argv[1:4]
libraries = [ctypes.CDLL(str(pathlib.Path(p).resolve())) for p in (old, new)]
functions = {}
for op, export in [('encode', 'size_repeat_encode'), ('decode', 'size_repeat_decode_prevalidated')]:
    functions[op] = [getattr(lib, export) for lib in libraries]
    for f in functions[op]:
        f.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t]
        f.restype = ctypes.c_size_t

def batch(f, source, source_len, output, count):
    start = time.process_time_ns()
    result = f(source, source_len, output, len(output), count)
    elapsed = time.process_time_ns() - start
    assert result <= len(output), 'conversion failed'
    return elapsed, result

rows = []
sizes = [int(n) for n in sys.argv[4:]] or [16, 22, 31, 32, 33, 63, 64, 65, 128, 256, 512, 4096, 65536, 1048576]
for size, zeros in [(n, 0) for n in sizes] + [(65, 63), (1024, 1008), (512, 512)]:
    rng = random.Random(size)
    data = bytes([0] * zeros + [rng.randrange(1, 256) for _ in range(size - zeros)])
    source = ctypes.create_string_buffer(data)
    output = ctypes.create_string_buffer(2 * size + 16)
    _, n = batch(functions['encode'][0], source, size, output, 1)
    text = output.raw[:n]
    for op in ['encode', 'decode']:
        value, expected = (data, text) if op == 'encode' else (text, data)
        source = ctypes.create_string_buffer(value)
        for f in functions[op]:
            _, n = batch(f, source, len(value), output, 1)
            assert output.raw[:n] == expected, (op, size, zeros)
        count = 1
        while min(batch(f, source, len(value), output, count)[0] for f in functions[op]) < 10_000_000:
            count *= 2
        for run in range(3):
            samples = [[], []]
            for sample in range(9):
                for position in range(2):
                    variant = (run + sample + position) % 2
                    elapsed, n = batch(functions[op][variant], source, len(value), output, count)
                    assert n == len(expected)
                    samples[variant].append(elapsed / count)
            for variant, times in enumerate(samples):
                rows.append(dict(operation=op, size=size, leading_zeros=zeros, run=run,
                                 variant=['baseline', 'candidate'][variant], median_ns=statistics.median(times),
                                 min_ns=min(times), max_ns=max(times), samples_ns=times, iterations=count))
        pathlib.Path(destination).write_text(json.dumps(rows, indent=2) + '\n')
        old_ns, new_ns = [statistics.median(r['median_ns'] for r in rows if r['operation'] == op and r['size'] == size and r['leading_zeros'] == zeros and r['variant'] == v) for v in ['baseline', 'candidate']]
        print(f'{op:6} {size:7} zeros={zeros:4} {old_ns:12.2f} -> {new_ns:12.2f} ns ({100*(1-new_ns/old_ns):+.1f}%)', flush=True)
