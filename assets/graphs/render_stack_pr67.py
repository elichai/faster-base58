#!/usr/bin/env python3
"""Render the frozen native five-PR stack study into a new output directory."""
import argparse
import csv
import json
import math
import os
from pathlib import Path
import tempfile

ROOT = Path(__file__).resolve().parent
PREFIX = 'native-stack-pr67-20261112-'
SIZES = (0, 1, 32, 52, 64, 65, 128, 256, 512, 1024, 1504, 1875, 2048, 4096, 8192, 16384,
         24576, 28672, 32768, 36864, 49152, 65536, 131072, 147456, 262144, 1048576, 4194304)
HOSTS = {'m5': 'Apple M5 Pro', 'gcp': 'AMD EPYC 9B45 · GCP'}

def load(data):
    rows = {name: list(csv.DictReader((data / (PREFIX + name + '.csv')).open()))
            for name in ('warm', 'cold', 'sizes')}
    repeat = data / (PREFIX + 'repeat.csv')
    rows['repeat'] = list(csv.DictReader(repeat.open())) if repeat.exists() else []
    assert len(rows['repeat']) in (0, 32)
    balanced = data / (PREFIX + 'balanced.csv')
    rows['balanced'] = list(csv.DictReader(balanced.open())) if balanced.exists() else []
    assert len(rows['balanced']) in (0, 16)
    manifest = json.loads((data / (PREFIX + 'sources.json')).read_text())
    assert [len(rows[k]) for k in ('warm', 'cold', 'sizes')] == [864, 64, 8]
    expected = {(host, lto, cache, label, op, n) for host in HOSTS for lto in ('fat', 'off')
                for cache in ('default', 'cache') for label in ('head', 'ctrl')
                for op in ('encode', 'decode') for n in SIZES}
    keys = [tuple(r[k] for k in ('platform', 'lto', 'cache', 'candidate', 'operation')) + (int(r['decoded_bytes']),) for r in rows['warm']]
    assert len(keys) == len(set(keys)) and set(keys) == expected
    cold_keys = [(r['platform'], r['lto'], r['cache'], r['candidate'], int(r['input_chars'])) for r in rows['cold']]
    assert len(set(cold_keys)) == len(cold_keys) and set(cold_keys) == {
        (host, lto, cache, label, n) for host in HOSTS for lto in ('fat', 'off')
        for cache in ('default', 'cache') for label in ('head', 'ctrl') for n in (1408, 2816, 5632, 32768)}
    size_keys = [tuple(r[k] for k in ('platform', 'lto', 'cache')) for r in rows['sizes']]
    assert len(set(size_keys)) == len(size_keys) and set(size_keys) == {
        (host, lto, cache) for host in HOSTS for lto in ('fat', 'off') for cache in ('default', 'cache')}
    for cohort in ('repeat', 'balanced'):
        if rows[cohort]:
            keys = [tuple(r[k] for k in ('platform', 'lto', 'cache', 'candidate', 'operation')) +
                    (int(r['decoded_bytes']),) for r in rows[cohort]]
            configurations = [('gcp', 'off', 'default', (32, 52, 64, 65))]
            if cohort == 'repeat':
                configurations += [('m5', 'fat', 'cache', (512, 1024, 2048, 32768))]
            expected = {(host, lto, cache, label, op, n) for host, lto, cache, sizes in configurations
                        for label in ('head', 'ctrl') for op in ('encode', 'decode') for n in sizes}
            assert len(keys) == len(set(keys)) and set(keys) == expected
    for r in rows['warm'] + rows['cold'] + rows['repeat'] + rows['balanced']:
        values = [float(r[k]) for k in ('ratio_min', 'ratio', 'ratio_max')]
        assert all(math.isfinite(v) and v > 0 for v in values) and values == sorted(values)
        assert r['baseline_commit'] == manifest['revisions']['main']
        assert r['candidate_commit'] == manifest['revisions']['head' if r['candidate'] == 'head' else 'main']
    for r in rows['sizes']:
        assert r['baseline_commit'] == manifest['revisions']['main']
        assert r['candidate_commit'] == manifest['revisions']['head']
    return rows, manifest

def ratio(row):
    return f"{float(row['ratio']):.3f} [{float(row['ratio_min']):.3f}, {float(row['ratio_max']):.3f}]"

def tables(rows):
    out = []
    for host in HOSTS:
        for lto in ('fat', 'off'):
            cells = {(r['cache'], r['operation'], int(r['decoded_bytes'])): r for r in rows['warm']
                     if (r['platform'], r['lto'], r['candidate']) == (host, lto, 'head')}
            out += [f'<details>\n<summary>{HOSTS[host]} / {lto}: all paired ratios</summary>', '',
                    '| Decoded B | Default encode | Default decode | Cache encode | Cache decode |',
                    '| ---: | ---: | ---: | ---: | ---: |']
            for n in SIZES:
                out.append('| ' + f'{n:,}' + ' | ' + ' | '.join(ratio(cells[c, op, n]) for c in ('default', 'cache') for op in ('encode', 'decode')) + ' |')
            out += ['', '</details>', '']
    return '\n'.join(out)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True, help='ignored measurement directory containing the frozen CSVs and manifest')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    rows, manifest = load(args.data)
    args.output.mkdir(parents=True, exist_ok=False)
    (args.output / 'ratio-tables.md').write_text(tables(rows))
    with tempfile.TemporaryDirectory(prefix='base58-stack67-plot-') as cache:
        os.environ.setdefault('MPLCONFIGDIR', cache)
        import matplotlib
        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 10,
                            'axes.spines.top': False, 'axes.spines.right': False,
                            'svg.fonttype': 'path', 'svg.hashsalt': 'native-stack-pr67-20261112'})
        for host, title in HOSTS.items():
            fig, axes = plt.subplots(2, 2, figsize=(12.5, 8.5), sharex=True)
            fig.subplots_adjust(left=.08, right=.985, bottom=.20, top=.85, hspace=.3, wspace=.2)
            for i, lto in enumerate(('fat', 'off')):
                for j, op in enumerate(('encode', 'decode')):
                    ax = axes[i, j]
                    ax.axhline(1, color='#6b7280', linestyle='--', linewidth=1)
                    for policy, color in [('default', '#1665ad'), ('cache', '#8057ae')]:
                        for label in ('head', 'ctrl'):
                            cells = sorted((r for r in rows['warm'] if
                                            (r['platform'], r['lto'], r['operation'], r['cache'], r['candidate'])
                                            == (host, lto, op, policy, label) and int(r['decoded_bytes']) >= 32),
                                           key=lambda r: int(r['decoded_bytes']))
                            x, y = [int(r['decoded_bytes']) for r in cells], [float(r['ratio']) for r in cells]
                            head = label == 'head'
                            ax.plot(x, y, color=color, marker='o' if head else 'x', markersize=3 if head else 2,
                                    linewidth=1.2 if head else .7, alpha=1 if head else .35,
                                    linestyle='-' if head else ':', label=policy.capitalize() + (' stack' if head else ' control'))
                            ax.errorbar(x, y, yerr=[[v - float(r['ratio_min']) for v, r in zip(y, cells)],
                                                   [float(r['ratio_max']) - v for v, r in zip(y, cells)]],
                                        fmt='none', color=color, alpha=.24 if head else .10, capsize=2, linewidth=.7)
                    for cohort, color, marker, name in [('repeat', '#c05216', 'D', 'Focused repeat'),
                                                       ('balanced', '#1b806c', 's', 'Full rotation')]:
                        for label in ('head', 'ctrl'):
                            repeated = [r for r in rows[cohort] if
                                        (r['platform'], r['lto'], r['operation'], r['candidate']) == (host, lto, op, label)]
                            if repeated:
                                x = [int(r['decoded_bytes']) for r in repeated]
                                y = [float(r['ratio']) for r in repeated]
                                ax.errorbar(x, y, yerr=[[v - float(r['ratio_min']) for v, r in zip(y, repeated)],
                                                       [float(r['ratio_max']) - v for v, r in zip(y, repeated)]],
                                            fmt=marker if label == 'head' else 'x', color=color, markersize=4,
                                            capsize=2, linewidth=.7, zorder=6,
                                            label=name + (' cache' if host == 'm5' else ' default')
                                            + (' stack' if label == 'head' else ' control'))
                    ax.set(xscale='log', title=f"{op.capitalize()} · {'fat LTO' if lto == 'fat' else 'LTO off'}")
                    ax.set_xticks([32, 512, 4096, 32768, 262144, 4194304])
                    ax.set_xticklabels(['32 B', '512 B', '4 KiB', '32 KiB', '256 KiB', '4 MiB'], rotation=25, ha='right')
                    ax.grid(axis='y', alpha=.18)
                    if j == 0:
                        ax.set_ylabel('Full stack / main · lower is faster')
            fig.suptitle('Five-PR stack versus main · ' + title, y=.96, fontsize=16, fontweight='bold')
            rev = manifest['revisions']
            fig.text(.5, .905, f"2026-11-12 · main {rev['main'][:12]} → stack {rev['head'][:12]} · includes PR67", ha='center')
            legend = {}
            for ax in axes.flat:
                handles, labels = ax.get_legend_handles_labels()
                legend.update(zip(labels, handles))
            fig.legend(legend.values(), legend.keys(), loc='lower center', bbox_to_anchor=(.5, .103), ncol=4, frameon=False, fontsize=9)
            fig.text(.5, .022, 'Public Auto · generic CPU flags · 5 processes × 3 copies × 3 batches · process CPU time\n'
                     'Points: median paired process ratios · bars: observed process range, not confidence intervals\n'
                     'Dotted controls: exact same main artifact · empty/one-byte cells retained in the guide\n'
                     'Diamonds: 7-process ×5-copy repeat; squares: 9-process ×3-copy full rotation; both15ms · controls: crosses',
                     ha='center', va='bottom', fontsize=9)
            fig.savefig(args.output / f'{host}-public.svg', metadata={'Date': None})
            fig.savefig(args.output / f'{host}-public.png', dpi=140)
            plt.close(fig)

if __name__ == '__main__':
    main()
