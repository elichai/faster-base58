"""Documentation checks and public timing/size table helpers; standard library only."""
import csv
import math
import re
from pathlib import Path
from urllib.parse import unquote, urlsplit
import xml.etree.ElementTree as ET

PLATFORMS = {'m5': 'Apple M5 Pro', 'gcp': 'AMD EPYC 9B45 (generic x86-64)',
             'wasm': 'WebAssembly on Apple M5 Pro (Node/V8)'}
SERIES = [('auto', 'default', 'Auto'), ('auto', 'cache', 'Auto + cache'),
          ('no-alloc', 'default', 'No allocation'), ('bs58', 'default', 'bs58'),
          ('five8', 'default', 'five8')]
TABLE_SIZES = [32, 64, 512, 4096, 65536, 1048576, 4194304]
SIZE_METRICS = {'native': ('code_bytes', 'readonly_bytes'),
                'wasm': ('wasm_code_section_bytes', 'wasm_data_section_bytes')}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read(path):
    with path.open(newline='', encoding='utf-8') as stream:
        reader = csv.DictReader(stream)
        require(reader.fieldnames and len(reader.fieldnames) == len(set(reader.fieldnames)),
                f'Missing/duplicate CSV header: {path}')
        rows = list(reader)
        require(all(None not in row and all(value is not None for value in row.values()) for row in rows),
                f'Malformed CSV row: {path}')
        return rows


def timing_cells(rows):
    """`scripts/public_bench.py` rows keyed by (platform, cache, backend, operation, size)."""
    cells, sources = {}, {}
    for row in rows:
        try:
            key = (row['platform'], row['cache'], row['backend'], row['operation'], int(row['decoded_bytes']))
            values = [float(row[name]) for name in ['process_min_ns', 'warm_ns', 'process_max_ns']]
            source = (row['measurement_date'], row['source_commit'], row['clock'])
        except (KeyError, ValueError) as error:
            raise ValueError(f'Malformed timing row: {error}') from error
        require(key[0] in PLATFORMS, f'Unknown timing platform: {key[0]}')
        require(key not in cells, f'Duplicate timing cell: {key}')
        require(all(math.isfinite(value) and value > 0 for value in values) and values == sorted(values),
                f'Invalid timing range: {key}')
        require(source[2] == ('elapsed' if key[0] == 'wasm' else 'process CPU'), f'Wrong timing clock: {key}')
        require(sources.setdefault(key[0], source) == source, f'Mixed timing sources: {key[0]}')
        cells[key] = row
    return cells


def size_deltas(paths):
    """Linked bytes above the cohort's empty probe, keyed by (platform, cache, backend).

    `paths` maps PLATFORM or PLATFORM+cache to a `scripts/code_size.py` sizes.csv."""
    deltas = {}
    for spec, path in paths.items():
        platform, _, cache = spec.partition('+')
        require(platform in PLATFORMS and cache in ('', 'cache'), f'Bad size cohort: {spec}')
        rows = [row for row in read(Path(path)) if (row['direction'], row['kind']) == ('both', 'cdylib')]
        probes = {row['backend']: row for row in rows}
        require(len(probes) == len(rows) and 'empty' in probes, f'Need one empty and one row per backend: {spec}')
        metrics = SIZE_METRICS['wasm' if platform == 'wasm' else 'native']
        for backend, row in probes.items():
            if backend != 'empty':
                deltas[platform, cache or 'default', backend] = [
                    int(row[metric]) - int(probes['empty'][metric]) for metric in metrics]
    return deltas


def duration(value):
    if value is None:
        return '—'
    scale, unit = (1e9, 's') if value >= 1e9 else (1e6, 'ms') if value >= 1e6 else (1e3, 'µs') if value >= 1e3 else (1, 'ns')
    return f'{value / scale:.3g} {unit}'


def payload(value):
    return f'{value // 1048576} MiB' if value >= 1048576 else f'{value // 1024} KiB' if value >= 1024 else f'{value} B'


def tables(timing_rows, deltas):
    """README timing tables per measured platform, then the linked-size table."""
    cells = timing_cells(timing_rows)
    platforms = [platform for platform in PLATFORMS if any(key[0] == platform for key in cells)]
    result = []
    for platform in platforms:
        result += [f'### {PLATFORMS[platform]}', '', '| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |',
                   '|---|---:|---:|---:|---:|---:|']
        for size in TABLE_SIZES:
            values = [[float(cells[platform, cache, backend, operation, size]['warm_ns'])
                       if (platform, cache, backend, operation, size) in cells else None
                       for operation in ['encode', 'decode']] for backend, cache, _ in SERIES]
            minima = [min((pair[index] for pair in values if pair[index] is not None), default=None) for index in [0, 1]]
            formatted = [' / '.join('**' + duration(value) + '**' if value is not None and value == minima[index] else duration(value)
                                   for index, value in enumerate(pair)) for pair in values]
            result.append('| ' + payload(size) + ' | ' + ' | '.join(formatted) + ' |')
        result.append('')
    sized = [platform for platform in PLATFORMS if any(key[0] == platform for key in deltas)]
    headers = {'m5': 'M5 code / readonly', 'gcp': 'x86-64 code / readonly', 'wasm': 'Wasm code / data section'}
    result += ['| Consumer | ' + ' | '.join(headers[platform] for platform in sized) + ' |',
               '|---|' + '---:|' * len(sized)]
    for backend, cache, label in SERIES[:4]:
        values = [' / '.join(f'{value / 1024:.1f} KiB' for value in deltas[platform, cache, backend])
                  if (platform, cache, backend) in deltas else '—' for platform in sized]
        result.append('| ' + label + ' | ' + ' | '.join(values) + ' |')
    return '\n'.join(result) + '\n'


def check(root):
    root = Path(root).resolve()
    # Explicit scope excludes unrelated untracked reviews and working notes.
    names = ['README.md', 'DESIGN.md', 'CORRECTNESS.md', 'REFERENCES.md', 'CI.md',
             'docs/README.md', 'docs/performance.md']
    documents = {}
    for name in names:
        path = root / name
        require(path.is_file(), f'Missing maintained document: {name}')
        text = path.read_text(encoding='utf-8')
        # Fenced examples do not create links in the rendered document.
        text = re.sub(r'^(```|~~~).*?^\1[^\n]*$', '', text, flags=re.M | re.S)
        # shortcut: repository link forms only; use a Markdown parser for nested destinations.
        targets = re.findall(r'!?\[[^\]\n]*\]\(\s*(<[^>]*>|[^\s)]+)', text)
        targets += re.findall(r'^\s*\[[^\]]+\]:\s*(<[^>]*>|\S+)', text, re.M)
        targets += re.findall(r'(?:href|src)=["\']([^"\']+)["\']', text)
        for target in targets:
            url = urlsplit(target.strip('<>'))
            if url.scheme or url.netloc or not url.path:
                continue
            linked = (path.parent / unquote(url.path)).resolve()
            require(linked.is_relative_to(root) and linked.exists(),
                    f'{name}: missing local link: {unquote(url.path)}')
        documents[name] = text
    guide = documents['README.md']
    for name in ['m5-public.svg', 'gcp-public.svg', 'wasm-public.svg']:
        require(re.search(r'!\[[^\]]*\]\(assets/graphs/' + re.escape(name) + r'\)', guide),
                f'README is missing selected figure: {name}')
    figures = list((root / 'assets/graphs').glob('*.svg'))
    for path in figures:
        try:
            svg = ET.fromstring(path.read_bytes())
        except ET.ParseError as error:
            raise ValueError(f'Invalid SVG: {path.name}: {error}') from error
        require(svg.tag == '{http://www.w3.org/2000/svg}svg', f'Invalid SVG root: {path.name}')
        require(not svg.findall('.//{http://www.w3.org/2000/svg}script'), f'Executable SVG: {path.name}')
    return dict(status='pass', documents=len(documents), svg_figures=len(figures),
                validation='local documentation links, selected figure references and SVG syntax; no measurement, numeric-table or pixel verification')
