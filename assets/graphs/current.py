"""Documentation checks and public CSV/table helpers, using the standard library."""
import csv
import math
import re
from pathlib import Path
from urllib.parse import unquote, urlsplit
import xml.etree.ElementTree as ET

SIZES = [0, 1, 8, 16, 21, 22, 23, 31, 32, 33, 63, 64, 65, 128, 256, 512, 1024,
         2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288, 1048576,
         2097152, 4194304]
SERIES = [('auto', 'default', 'Auto'), ('auto', 'cache', 'Auto + cache'),
          ('no-alloc', 'default', 'No allocation'), ('bs58', 'default', 'bs58'),
          ('five8', 'default', 'five8')]


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
    cells = {}
    sources = {}
    labels = {}
    for row in rows:
        try:
            key = (row['platform'], row['cache'], row['backend'], row['operation'], int(row['decoded_bytes']))
            require(key not in cells, f'Duplicate timing cell: {key}')
            require(key[0] in {'m5', 'gcp', 'wasm'}, f'Unknown public timing platform: {key[0]}')
            scope = 'ordinary public Wasm module; no __internals' if key[0] == 'wasm' else 'ordinary public consumer; no __internals'
            role = {'auto': 'public', 'no-alloc': 'public', 'bs58': 'competitor',
                    'five8': 'competitor', 'empty': 'scaffold'}.get(key[2])
            require(role is not None and row['role'] == role and row['scope'] == scope,
                    f'Wrong public timing scope: {key}')
            source = row['source_fingerprint']
            require(re.fullmatch(r'[0-9a-f]{64}', source), f'Invalid timing source fingerprint: {key}')
            require(sources.setdefault(key[0], source) == source, f'Mixed timing sources: {key[0]}')
            label = (row.get('measurement_date', ''), row.get('source_commit', ''))
            require(labels.setdefault(key[0], label) == label, f'Mixed timing source labels: {key[0]}')
            if row['status'] == 'measured':
                values = [float(row[name]) for name in ['process_min_ns', 'warm_ns', 'process_max_ns']]
                require(all(math.isfinite(value) and value > 0 for value in values) and values == sorted(values),
                        f'Invalid timing range: {key}')
                require(row['primary_clock'] == ('elapsed' if key[0] == 'wasm' else 'process CPU'),
                        f'Wrong timing clock: {key}')
                require(int(row['fresh_processes']) > 0 and int(row['warm_batches_per_process']) > 0,
                        f'Invalid timing sample counts: {key}')
            else:
                require(row['status'] == 'skipped' and row['reason'].strip(), f'Unexplained timing skip: {key}')
                require(all(not row[name] for name in ['warm_ns', 'process_min_ns', 'process_max_ns', 'primary_clock']),
                        f'Skipped timing cell contains measurements: {key}')
            cells[key] = row
        except (KeyError, TypeError) as error:
            raise ValueError(f'Missing timing field: {error}') from error
    platforms = ['m5', 'wasm'] + (['gcp'] if 'gcp' in sources else [])
    expected = {(platform, cache, backend, operation, size)
                for platform in platforms for cache in ['default', 'cache']
                for backend in ['empty', 'auto', 'no-alloc', 'bs58', 'five8']
                for operation in ['encode', 'decode'] for size in SIZES}
    require(set(cells) == expected, 'Missing/unexpected timing cells; provide the full grid with explicit explained skips')
    return cells


def duration(value):
    if value is None:
        return '—'
    scale, unit = (1e9, 's') if value >= 1e9 else (1e6, 'ms') if value >= 1e6 else (1e3, 'µs') if value >= 1e3 else (1, 'ns')
    return f'{value / scale:.3g} {unit}'


def payload(value):
    return f'{value // 1048576} MiB' if value >= 1048576 else f'{value // 1024} KiB' if value >= 1024 else f'{value} B'


def tables(public_rows, size_rows):
    cells = {key: row for key, row in timing_cells(public_rows).items() if row['status'] == 'measured'}
    result = []
    platforms = [('m5', 'Apple M5 Pro')]
    if any(row['platform'] == 'gcp' for row in public_rows):
        platforms.append(('gcp', 'AMD EPYC 9B45 (generic x86-64)'))
    platforms.append(('wasm', 'WebAssembly on Apple M5 Pro (Node/V8)'))
    for platform, title in platforms:
        result += [f'### {title}', '', '| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |',
                   '|---|---:|---:|---:|---:|---:|']
        for size in [32, 64, 512, 4096, 65536, 1048576, 4194304]:
            values = [[float(cells[platform, cache, backend, operation, size]['warm_ns'])
                       if (platform, cache, backend, operation, size) in cells else None
                       for operation in ['encode', 'decode']] for backend, cache, _ in SERIES]
            minima = [min((pair[index] for pair in values if pair[index] is not None), default=None) for index in [0, 1]]
            formatted = [' / '.join('**' + duration(value) + '**' if value is not None and value == minima[index] else duration(value)
                                   for index, value in enumerate(pair)) for pair in values]
            result.append('| ' + payload(size) + ' | ' + ' | '.join(formatted) + ' |')
        result += ['', 'Each cell is **encode / decode**. Bold marks the lowest observed median, not statistical significance.', '']
    headers = ['M5 code / readonly']
    if len(platforms) == 3:
        headers.append('x86-64 code / readonly')
    headers.append('Wasm code / data section')
    result += ['### Linked budgets above the empty probe', '', '| Consumer | ' + ' | '.join(headers) + ' |',
               '|---|' + '---:|' * len(headers)]
    for backend, cache, label in SERIES[:4]:
        values = []
        for platform, _ in platforms:
            matches = [row for row in size_rows if (row['platform'], row['cache'], row['backend'], row['direction']) == (platform, cache, backend, 'both')]
            require(len(matches) == 1, f'Missing/duplicate linked-size cell: {platform}/{cache}/{backend}')
            row = matches[0]
            metrics = ['delta_empty_wasm_code_section_bytes', 'delta_empty_wasm_data_section_bytes'] if platform == 'wasm' else ['delta_empty_code_bytes', 'delta_empty_readonly_bytes']
            values.append(' / '.join(f'{int(row[key]) / 1024:.1f} KiB' for key in metrics))
        result.append('| ' + label + ' | ' + ' | '.join(values) + ' |')
    result += ['', 'Each timing cell is measured or explicitly skipped in the supplied CSV. Linked probe deltas include associated code and metadata; they are not standalone rlib sizes.', '']
    return '\n'.join(result)


def check(root):
    root = Path(root).resolve()
    # Explicit scope excludes unrelated untracked reviews and working notes.
    names = ['README.md', 'DESIGN.md', 'CORRECTNESS.md', 'REFERENCES.md',
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
        local = set()
        for target in targets:
            url = urlsplit(target.strip('<>'))
            if url.scheme or url.netloc or not url.path:
                continue
            linked = (path.parent / unquote(url.path)).resolve()
            require(linked.is_relative_to(root) and linked.exists(),
                    f'{name}: missing local link: {unquote(url.path)}')
            local.add(linked)
        documents[name] = (text, local)
    guide = documents['README.md'][0]
    for name in ['m5-public.svg', 'wasm-public.svg']:
        require(re.search(r'!\[[^\]]*\]\(assets/graphs/' + re.escape(name) + r'\)', guide),
                f'README is missing selected figure: {name}')
    require(any(re.search(r'!\[[^\]]*\]\(assets/graphs/' + re.escape(name) + r'\)', guide)
                for name in ['gcp-public.svg', 'gcp-generic-backends.svg']),
            'README is missing the selected x86-64 figure')
    native = root / 'assets/graphs/gcp-native-backends.svg'
    if native.exists():
        require(native in documents['docs/performance.md'][1], 'Missing native figure in performance documentation')
    require(all(native not in links for name, (_, links) in documents.items() if name != 'docs/performance.md'),
            'The native figure belongs only in performance documentation')
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
