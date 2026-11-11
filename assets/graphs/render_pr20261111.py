#!/usr/bin/env python3
"""Render the frozen 2026-11-11 native studies; never build or run benchmarks.

Input summaries and raw-input hashes live in data/native-20261111*. Outputs go
only to a new directory. Requires requirements.txt for figures; table helpers
use the standard library. Empty and one-byte cells remain in the input CSV.
"""
import argparse
import math
import os
from pathlib import Path
import tempfile

from current import duration, read, require

ROOT = Path(__file__).resolve().parent
STUDIES = {
    'stack': ('Full stack', '9825212e9b9223f13cfae73905357e7877ded357',
              'd82345997505864abd732a7f4b6d497827072564'),
    'pr58': ('Isolated #58', '62ede32a59814a3385383dd93ae8d118be5bf284',
             '9626df5a899836a9c63c58c2c2621810050e432d'),
}
HOSTS = {'m5': 'Apple M5 Pro', 'gcp': 'AMD EPYC 9B45 · GCP'}
TABLE_SIZES = (32, 512, 4096, 32768, 1048576, 4194304)


def load():
    rows = read(ROOT / 'data/native-20261111.csv')
    sizes = read(ROOT / 'data/native-20261111-sizes.csv')
    seen = set()
    for row in rows:
        key = tuple(row[k] for k in ('study', 'cohort', 'platform', 'lto', 'cache', 'operation', 'decoded_bytes'))
        require(key not in seen, f'Duplicate timing cell: {key}')
        seen.add(key)
        require(row['study'] in STUDIES and row['platform'] in HOSTS, f'Unknown study/host: {key}')
        require(row['lto'] in ('fat', 'off') and row['cache'] in ('default', 'cache'), f'Unknown configuration: {key}')
        require(row['operation'] in ('encode', 'decode'), f'Unknown operation: {key}')
        require(row['measurement_date'] == '2026-11-11', f'Wrong date: {key}')
        require((row['baseline_commit'], row['candidate_commit']) == STUDIES[row['study']][1:], f'Mixed sources: {key}')
        values = [float(row[k]) for k in ('ratio_min', 'ratio', 'ratio_max')]
        require(all(math.isfinite(v) and v > 0 for v in values) and values == sorted(values), f'Invalid ratio range: {key}')
        require(all(math.isfinite(float(row[k])) and float(row[k]) > 0 for k in ('main_ns', 'candidate_ns')), f'Invalid absolute time: {key}')
    require(len(rows) == 790 and len(sizes) == 16, 'Incomplete frozen input summaries')
    expected = {(study, host, lto, cache) for study in STUDIES for host in HOSTS
                for lto in ('fat', 'off') for cache in ('default', 'cache')}
    require({tuple(row[k] for k in ('study', 'platform', 'lto', 'cache')) for row in sizes} == expected,
            'Missing/duplicate size configurations')
    return rows, sizes


def payload(value):
    if value >= 1048576 and value % 1048576 == 0:
        return f'{value // 1048576} MiB'
    if value >= 1024 and value % 1024 == 0:
        return f'{value // 1024} KiB'
    return f'{value:,} B'


def ratio(row):
    return f"{float(row['ratio']):.3f} [{float(row['ratio_min']):.3f}, {float(row['ratio_max']):.3f}]"


def tables(rows, sizes, study):
    result = []
    for host in HOSTS:
        for lto in ('fat', 'off'):
            cells = {(r['cache'], r['operation'], int(r['decoded_bytes'])): r for r in rows
                     if (r['study'], r['cohort'], r['platform'], r['lto']) == (study, 'primary', host, lto)}
            result += [f"#### {HOSTS[host]}, {'fat LTO' if lto == 'fat' else 'LTO off'}", '',
                       '| Payload | Default encode | Default decode | Cache encode | Cache decode |',
                       '| --- | ---: | ---: | ---: | ---: |']
            for n in TABLE_SIZES:
                values = [f"{duration(float(cells[c, op, n]['main_ns']))} → {duration(float(cells[c, op, n]['candidate_ns']))}"
                          for c in ('default', 'cache') for op in ('encode', 'decode')]
                result.append('| ' + payload(n) + ' | ' + ' | '.join(values) + ' |')
            result += ['', '<details>', '<summary>All paired warm ratios, median [min, max]</summary>', '',
                       '| Payload | Default encode | Default decode | Cache encode | Cache decode |',
                       '| --- | ---: | ---: | ---: | ---: |']
            for n in sorted({key[2] for key in cells if key[2] >= 32}):
                result.append('| ' + payload(n) + ' | ' + ' | '.join(ratio(cells[c, op, n])
                              for c in ('default', 'cache') for op in ('encode', 'decode')) + ' |')
            result += ['', '</details>', '']
    result += ['#### Linked sections above the matching empty consumer', '',
               '| Host / LTO / policy | Code B, main → candidate (Δ) | Readonly B, main → candidate (Δ) | Data / BSS Δ B | Stripped file B, main → candidate (Δ) |',
               '| --- | ---: | ---: | ---: | ---: |']
    for r in sizes:
        if r['study'] != study:
            continue
        def pair(metric):
            a, b = int(r['main_' + metric]), int(r['candidate_' + metric])
            return f'{a:,} → {b:,} ({b-a:+,})'
        delta = lambda k: int(r['candidate_' + k]) - int(r['main_' + k])
        result.append(f"| {r['platform']} / {r['lto']} / {r['cache']} | {pair('code_bytes')} | {pair('readonly_bytes')} | "
                      f"{delta('data_bytes'):+,} / {delta('bss_bytes'):+,} | {pair('stripped_file_bytes')} |")
    return '\n'.join(result) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='new output directory; refuse overwrite')
    args = parser.parse_args()
    rows, sizes = load()
    args.output.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix='base58-pr-plot-') as cache:
        os.environ.setdefault('MPLCONFIGDIR', cache)
        os.environ.setdefault('XDG_CACHE_HOME', cache)
        import matplotlib
        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 10, 'axes.spines.top': False,
                            'axes.spines.right': False, 'svg.fonttype': 'path', 'svg.hashsalt': 'base58-pr-20261111'})
        for study, (title, baseline, candidate) in STUDIES.items():
            (args.output / f'{study}-tables.md').write_text(tables(rows, sizes, study))
            for host, host_title in HOSTS.items():
                primary = [r for r in rows if (r['study'], r['cohort'], r['platform']) == (study, 'primary', host)
                           and int(r['decoded_bytes']) >= 32]
                low, high = min(float(r['ratio_min']) for r in primary), max(float(r['ratio_max']) for r in primary)
                margin = max(.012, (high - low) * .08)
                fig, axes = plt.subplots(2, 2, figsize=(12.5, 8.5), sharex=True, sharey=True)
                fig.subplots_adjust(left=.075, right=.985, bottom=.20, top=.85, hspace=.28, wspace=.15)
                for i, lto in enumerate(('fat', 'off')):
                    for j, op in enumerate(('encode', 'decode')):
                        ax = axes[i, j]
                        ax.axhline(1, color='#6b7280', linewidth=1, linestyle='--')
                        for policy, color, label in [('default', '#1665ad', 'Default'), ('cache', '#8057ae', 'Cache')]:
                            cells = sorted((r for r in primary if (r['lto'], r['operation'], r['cache']) == (lto, op, policy)),
                                           key=lambda r: int(r['decoded_bytes']))
                            x, y = [int(r['decoded_bytes']) for r in cells], [float(r['ratio']) for r in cells]
                            ax.plot(x, y, color=color, label=label, marker='o', markersize=3, linewidth=1.3)
                            ax.errorbar(x, y, yerr=[[v - float(r['ratio_min']) for v, r in zip(y, cells)],
                                                   [float(r['ratio_max']) - v for v, r in zip(y, cells)]],
                                        fmt='none', ecolor=color, alpha=.3, capsize=2, linewidth=.8)
                        repeat = [r for r in rows if (r['study'], r['cohort'], r['platform'], r['lto'], r['operation'])
                                  == (study, 'repeat32', host, lto, op)]
                        if repeat:
                            ax.scatter([int(r['decoded_bytes']) for r in repeat], [float(r['ratio']) for r in repeat],
                                       marker='D', s=28, color='#c05216', label='Focused repeat', zorder=6)
                        ax.set(xscale='log', ylim=(min(low, 1) - margin, max(high, 1) + margin),
                               title=f"{op.capitalize()} · {'fat LTO' if lto == 'fat' else 'LTO off'}")
                        ax.grid(axis='y', alpha=.18)
                        ax.set_xticks([32, 512, 4096, 32768, 262144, 4194304])
                        ax.set_xticklabels(['32 B', '512 B', '4 KiB', '32 KiB', '256 KiB', '4 MiB'], rotation=25, ha='right')
                        if j == 0:
                            ax.set_ylabel('Candidate / main · lower is faster')
                        if i == 1:
                            ax.set_xlabel('Decoded payload size')
                fig.suptitle(f'{title} versus main · {host_title}', y=.96, fontsize=16, fontweight='bold')
                fig.text(.5, .905, f'2026-11-11 · baseline {baseline[:12]} → candidate {candidate[:12]}', ha='center', fontsize=10)
                handles, labels = axes[1, 0].get_legend_handles_labels()
                fig.legend(handles, labels, loc='lower center', bbox_to_anchor=(.5, .095), ncol=3, frameon=False)
                footer = 'Native public Auto · process CPU time · 5 processes × 3 copies × 3 batches\n'
                footer += 'Paired medians; bars: observed process range, not confidence intervals · 1.0 = main'
                if study == 'stack' and host == 'gcp':
                    footer += '\nDiamonds: separate 7-process × 5-copy repeat; original 32 B outlier is retained'
                fig.text(.5, .018, footer, ha='center', va='bottom', fontsize=9)
                name = f"{host}-{'public' if study == 'stack' else 'pr58'}"
                fig.savefig(args.output / f'{name}.png', dpi=140)
                fig.savefig(args.output / f'{name}.svg', metadata={'Title': f'{title}: {host_title}', 'Date': None})
                plt.close(fig)
    print(args.output)


if __name__ == '__main__':
    main()
