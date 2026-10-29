"""Compare exact parent/candidate encoding, optionally reversing library load order."""
import argparse
import ctypes
import json
import random
import statistics
import time
from pathlib import Path

if not __debug__:
    raise SystemExit('benchmark requires Python assertions; omit -O')

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('baseline')
parser.add_argument('candidate')
parser.add_argument('output', type=Path)
parser.add_argument('--reverse', action='store_true')
parser.add_argument('--sizes', nargs='+', type=int,
                    default=[8191, 8192, 8193, 12288, 32768, 65535, 65536, 65537,
                             262143, 262144, 262145, 1048575, 1048576, 1048577, 4194304])
args = parser.parse_args()
if any(n <= 0 for n in args.sizes):
    parser.error('sizes must be positive')
assert not args.output.exists()
paths = {'baseline': args.baseline, 'candidate': args.candidate}
libraries, functions = {}, {}
order = ['candidate', 'baseline'] if args.reverse else ['baseline', 'candidate']
for name in order:
    libraries[name] = ctypes.CDLL(str(Path(paths[name]).resolve()))
    f = libraries[name].size_repeat_encode
    f.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t]
    f.restype = ctypes.c_size_t
    functions[name] = f
addresses = {name: ctypes.cast(f, ctypes.c_void_p).value for name, f in functions.items()}
assert len(set(addresses.values())) == 2, 'both paths resolved to the same loaded function'

def batch(f, source, n, output, iterations):
    start = time.process_time_ns()
    length = f(source, n, output, len(output), iterations)
    elapsed = time.process_time_ns() - start
    assert length <= len(output)
    return elapsed, length

rows = []
for n in args.sizes:
    data = bytearray(random.Random(n).randbytes(n))
    data[0] |= 1
    source = ctypes.create_string_buffer(bytes(data))
    output = ctypes.create_string_buffer(2 * n + 32)
    _, length = batch(functions['baseline'], source, n, output, 1)
    expected = output.raw[:length]
    for f in functions.values():
        _, got = batch(f, source, n, output, 1)
        assert got == length and output.raw[:got] == expected
    iterations = 1
    while min(batch(f, source, n, output, iterations)[0] for f in functions.values()) < 10_000_000:
        iterations *= 2
    for trial in range(3):
        samples = {name: [] for name in functions}
        for sample in range(9):
            names = order if (trial + sample) % 2 == 0 else order[::-1]
            for name in names:
                elapsed, got = batch(functions[name], source, n, output, iterations)
                assert got == length
                samples[name].append(elapsed / iterations)
        for name, values in samples.items():
            rows.append(dict(size=n, trial=trial, variant=name, library=paths[name],
                             iterations=iterations, median_ns=statistics.median(values), samples_ns=values))
    args.output.write_text(json.dumps(dict(load_order=order, function_addresses=addresses, rows=rows), indent=2) + '\n')
    old, new = [statistics.median(r['median_ns'] for r in rows if r['size'] == n and r['variant'] == name)
                for name in ('baseline', 'candidate')]
    print(f'{n}: {old/1e6:.6f} -> {new/1e6:.6f} ms ({100*(1-new/old):+.2f}%)', flush=True)
