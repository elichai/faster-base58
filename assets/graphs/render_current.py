#!/usr/bin/env python3
"""Render the README figures and tables from measurement CSVs; never run benchmarks.

--data takes scripts/public_bench.py CSVs. --sizes takes PLATFORM[+cache]=sizes.csv
from scripts/code_size.py cohorts that include the empty probe. Requires the pinned
Matplotlib environment (requirements.txt). Writes only to a new directory.
"""
import argparse
import os
from pathlib import Path
import tempfile

from current import PLATFORMS, SERIES, duration, read, size_deltas, tables


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, nargs='+', required=True, help='public_bench.py CSVs')
    parser.add_argument('--sizes', nargs='+', required=True, metavar='PLATFORM[+cache]=CSV',
                        help='code_size.py sizes.csv per cohort')
    parser.add_argument('--output', type=Path, required=True, help='new empty destination; never overwrite committed files')
    args = parser.parse_args()
    try:
        rows = [row for path in args.data for row in read(path)]
        generated_tables = tables(rows, size_deltas(dict(spec.split('=', 1) for spec in args.sizes)))
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, f'Invalid measurement input: {error}\n')
    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix='base58-plot-cache-') as cache:
        os.environ.setdefault('MPLCONFIGDIR', cache)
        os.environ.setdefault('XDG_CACHE_HOME', cache)
        import matplotlib
        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        from matplotlib.ticker import FuncFormatter
        colors = {'Auto': '#1665ad', 'Auto + cache': '#8057ae', 'No allocation': '#d68a19',
                  'bs58': '#454b54', 'five8': '#16866b'}
        plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 10, 'axes.titlesize': 12,
            'axes.spines.top': False, 'axes.spines.right': False, 'svg.fonttype': 'none',
            'svg.hashsalt': 'faster-base58-public'})
        for platform, title in PLATFORMS.items():
            measured = [row for row in rows if row['platform'] == platform]
            if not measured:
                continue
            figure, axes = plt.subplots(1, 2, figsize=(13, 6.4))
            figure.subplots_adjust(left=.075, right=.99, bottom=.25, top=.86, wspace=.24)
            for ax, operation in zip(axes, ['encode', 'decode']):
                for backend, policy, label in SERIES:
                    selected = sorted([row for row in measured if (row['cache'], row['backend'], row['operation'])
                        == (policy, backend, operation) and int(row['decoded_bytes']) > 0],
                        key=lambda row: int(row['decoded_bytes']))
                    x = [int(row['decoded_bytes']) for row in selected]
                    y = [float(row['warm_ns']) for row in selected]
                    five8 = backend == 'five8'
                    ax.plot(x, y, color=colors[label], label=label, linewidth=1.8, marker='D' if five8 else 'o',
                            markersize=5 if five8 else 2.8, linestyle='None' if five8 else '-',
                            alpha=.96, zorder=5 if five8 else 3)
                    ax.errorbar(x, y, yerr=[[value - float(row['process_min_ns']) for value, row in zip(y, selected)],
                                            [float(row['process_max_ns']) - value for value, row in zip(y, selected)]],
                                fmt='none', ecolor=colors[label], alpha=.3, elinewidth=.8, capsize=1.5)
                ax.set(xscale='log', yscale='log', title=operation.capitalize(), xlabel='Decoded payload size')
                ax.set_xticks([1, 32, 64, 512, 4096, 65536, 1048576, 4194304])
                ax.set_xticklabels(['1 B', '32 B', '64 B', '512 B', '4 KiB', '64 KiB', '1 MiB', '4 MiB'], rotation=28, ha='right')
                ax.yaxis.set_major_formatter(FuncFormatter(lambda value, _: duration(value)))
                ax.grid(which='major', alpha=.18, linewidth=.7)
                ax.grid(which='minor', alpha=.05, linewidth=.5)
                ax.set_ylabel('Time per conversion · lower is faster')
            figure.suptitle('Base58 conversion time · ' + title, fontsize=15, fontweight='bold', y=.97)
            handles, labels = axes[0].get_legend_handles_labels()
            figure.legend(handles, labels, loc='lower center', bbox_to_anchor=(.5, .075), ncol=5, frameon=False)
            identity = measured[0]
            figure.text(.5, .022, f"Public API · {identity['measurement_date']} · source {identity['source_commit'][:12]}"
                + ' · warm medians; bars: process range\n'
                + ('V8 elapsed time' if platform == 'wasm' else 'Native process CPU time')
                + ' · missing points were not measured', fontsize=9, ha='center', va='bottom')
            figure.savefig(destination / f'{platform}-public.svg', metadata={'Title': 'Base58 conversion time: ' + title, 'Date': None})
            plt.close(figure)
    (destination / 'public-tables.md').write_text(generated_tables, encoding='utf-8')
    print(destination)


if __name__ == '__main__':
    main()
