"""Validate the current public figures and table inputs using only the standard library.

The committed SVGs are hash checked. This does not regenerate Matplotlib pixels.
"""
import csv
import hashlib
import json
import math
from pathlib import Path
import xml.etree.ElementTree as ET

DATA = Path('docs/benchmarks/2026-10-27')
SIZES = [0, 1, 8, 16, 21, 22, 23, 31, 32, 33, 63, 64, 65, 128, 256, 512, 1024,
         2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288, 1048576,
         2097152, 4194304]
SERIES = [('auto', 'default', 'Auto'), ('auto', 'cache', 'Auto + cache'),
          ('no-alloc', 'default', 'No allocation'), ('bs58', 'default', 'bs58'),
          ('five8', 'default', 'five8')]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    with path.open(newline='') as stream:
        return list(csv.DictReader(stream))


def duration(value):
    if value is None:
        return '—'
    scale, unit = (1e9, 's') if value >= 1e9 else (1e6, 'ms') if value >= 1e6 else (1e3, 'µs') if value >= 1e3 else (1, 'ns')
    return f'{value / scale:.3g} {unit}'


def payload(value):
    return f'{value // 1048576} MiB' if value >= 1048576 else f'{value // 1024} KiB' if value >= 1024 else f'{value} B'


def tables(public_rows, size_rows):
    measured = [row for row in public_rows if row['status'] == 'measured']
    cells = {(row['platform'], row['cache'], row['backend'], row['operation'], int(row['decoded_bytes'])): row
             for row in measured}
    result = []
    for platform, title in [('m5', 'Apple M5 Pro'), ('wasm', 'WebAssembly on Apple M5 Pro (Node/V8)')]:
        result += [f'### {title}', '', '| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |',
                   '|---|---:|---:|---:|---:|---:|']
        for size in [32, 64, 512, 4096, 65536, 1048576, 4194304]:
            values = [[float(cells[platform, cache, backend, operation, size]['warm_ns'])
                       if (platform, cache, backend, operation, size) in cells else None
                       for operation in ['encode', 'decode']] for backend, cache, _ in SERIES]
            minima = [min(pair[index] for pair in values if pair[index] is not None) for index in [0, 1]]
            formatted = [' / '.join('**' + duration(value) + '**' if value == minima[index] else duration(value)
                                   for index, value in enumerate(pair)) for pair in values]
            result.append('| ' + payload(size) + ' | ' + ' | '.join(formatted) + ' |')
        result += ['', 'Each cell is **encode / decode**. Bold marks the lowest observed median, not statistical significance.', '']
    result += ['### Linked budgets above the empty probe', '', '| Consumer | M5 code / readonly | Wasm code / data section |',
               '|---|---:|---:|']
    for backend, cache, label in SERIES[:4]:
        values = []
        for platform in ['m5', 'wasm']:
            row = next(row for row in size_rows if (row['platform'], row['cache'], row['backend'], row['direction']) == (platform, cache, backend, 'both'))
            metrics = ['delta_empty_code_bytes', 'delta_empty_readonly_bytes'] if platform == 'm5' else ['delta_empty_wasm_code_section_bytes', 'delta_empty_wasm_data_section_bytes']
            values.append(' / '.join(f'{int(row[key]) / 1024:.1f} KiB' for key in metrics))
        result.append('| ' + label + ' | ' + ' | '.join(values) + ' |')
    result += ['', 'Both API directions, opt3/fat LTO. These linked probe deltas include associated code and metadata; they are not standalone rlib sizes.',
               'Cache adds 6,280 native writable static header bytes plus history-dependent heap tables. Heap and RSS are reported separately.', '']
    return '\n'.join(result)


def check(root):
    root = Path(root)
    manifest = json.loads((root / DATA / 'public-graphs.json').read_text())
    require(manifest['schema'] == 1, 'Unknown public graph manifest schema')
    for group in ['inputs', 'outputs']:
        for name, expected in manifest[group].items():
            path = root / name
            require(path.is_file() and digest(path) == expected, 'Current graph hash mismatch: ' + name)
    rows = read(root / DATA / 'public.csv')
    keys = [(row['platform'], row['cache'], row['backend'], row['operation'], int(row['decoded_bytes'])) for row in rows]
    expected_keys = {(platform, cache, backend, operation, size)
                     for platform in ['m5', 'wasm'] for cache in ['default', 'cache']
                     for backend in ['empty', 'auto', 'no-alloc', 'bs58', 'five8']
                     for operation in ['encode', 'decode'] for size in SIZES}
    require(len(keys) == len(set(keys)) and set(keys) == expected_keys, 'Missing/duplicate public graph data cells')
    for row in rows:
        require(row['source_fingerprint'] == manifest['source_fingerprints'][row['platform']], 'Public source epoch mismatch')
        expected_scope = 'ordinary public consumer; no __internals' if row['platform'] == 'm5' else 'ordinary public Wasm module; no __internals'
        require(row['scope'] == expected_scope, 'Public consumer scope mismatch')
        if row['status'] == 'measured':
            require(row['primary_clock'] == ('process CPU' if row['platform'] == 'm5' else 'elapsed'), 'Public clock mismatch')
            values = [float(row[name]) for name in ['process_min_ns', 'warm_ns', 'process_max_ns']]
            require(all(math.isfinite(value) and value > 0 for value in values) and values == sorted(values), 'Invalid public process range')
            require(row['fresh_processes'] == row['warm_batches_per_process'] == '3', 'Public sampling mismatch')
        else:
            require(row['status'] == 'skipped' and row['reason'], 'Unexplained public data skip')
            require(not row['warm_ns'] and not row['primary_clock'], 'Skipped public cell contains measurements')
    generated = tables(rows, read(root / DATA / 'figure-linked-sizes.csv'))
    require((root / DATA / 'public-tables.md').read_text() == generated, 'Current table output differs from numeric inputs')
    guide = (root / 'README.md').read_text()
    blocks = ['\n'.join(block.splitlines()[2:]) for block in generated.split('### ')[1:]]
    # Bind table values/formatting but leave prose and section names to the README editor.
    for block in blocks:
        lines = [line for line in block.splitlines() if line.startswith('|')]
        if lines and lines[0].startswith('| Consumer |'):
            x86_rows = read(root / DATA / 'gcp-boot2/linked-artifacts.csv')
            columns = [' x86-64 code / readonly ', '---:']
            for backend, cache, _ in SERIES[:4]:
                matches = [row for row in x86_rows if
                           (row['platform'], row['consumer_scope'], row['cache'], row['backend'], row['direction'], row['kind']) ==
                           ('gcp-generic', 'frozen all-backend sizes', cache, backend, 'both', 'cdylib')]
                require(len(matches) == 1, 'Missing/duplicate generic x86 linked-size cell')
                row = matches[0]
                values = ' / '.join(f'{int(row[key]) / 1024:.1f} KiB' for key in
                                    ['above_empty_code_bytes', 'above_empty_readonly_bytes'])
                columns.append(' ' + values + ' ')
            require(len(lines) == len(columns), 'Unexpected README linked-size row count')
            for index, column in enumerate(columns):
                fields = lines[index].split('|')
                fields.insert(3, column)
                lines[index] = '|'.join(fields)
        table = '\n'.join(lines)
        require(table and table in guide, 'README current table differs from public numeric inputs')
    for name in ['m5-public.svg', 'wasm-public.svg']:
        require('assets/graphs/' + name in guide, 'README is missing current public figure: ' + name)
        svg = ET.fromstring((root / 'assets/graphs' / name).read_bytes())
        require(svg.tag == '{http://www.w3.org/2000/svg}svg', 'Current public figure is not SVG')
        require(not svg.findall('.//{http://www.w3.org/2000/svg}script'), 'Unexpected executable SVG content')
    return dict(public_cells=len(rows), public_figures=2, current_readme_tables=3,
                validation='input/output hashes, source/scopes, numeric table regeneration, SVG parsing; not pixel regeneration')
