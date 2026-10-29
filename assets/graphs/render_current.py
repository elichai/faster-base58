#!/usr/bin/env python3
"""Render public figures/tables from external CSVs; never run benchmarks.

--data takes the public timing grid (including explained skipped cells).
--sizes takes linked probe sizes. See current.py for the accepted columns/grid.
Requires the optional pinned Matplotlib environment. Outputs are new files only.
"""
import argparse
import os
from pathlib import Path
import tempfile

from current import read, tables


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True, help='external public timing CSV')
    parser.add_argument('--sizes', type=Path, required=True, help='external linked-size CSV')
    parser.add_argument('--output', type=Path, required=True, help='new empty destination; never overwrite committed files')
    args = parser.parse_args()
    try:
        rows = read(args.data)
        generated_tables = tables(rows, read(args.sizes))
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
        from current import duration
        series = [('auto', 'default', 'Auto', '#1665ad'), ('auto', 'cache', 'Auto + cache', '#8057ae'),
                  ('no-alloc', 'default', 'No allocation', '#d68a19'), ('bs58', 'default', 'bs58', '#454b54'),
                  ('five8', 'default', 'five8', '#16866b')]
        plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 10, 'axes.titlesize': 12,
            'axes.spines.top': False, 'axes.spines.right': False, 'svg.fonttype': 'none',
            'svg.hashsalt': 'faster-base58-public'})
        platforms = [('m5', 'Apple M5 Pro')]
        if any(row['platform'] == 'gcp' for row in rows):
            platforms.append(('gcp', 'AMD EPYC 9B45 · generic x86-64'))
        platforms.append(('wasm', 'WebAssembly on Apple M5 Pro (Node/V8)'))
        for platform, title in platforms:
            figure, axes = plt.subplots(1, 2, figsize=(13, 6.4))
            figure.subplots_adjust(left=.075, right=.99, bottom=.25, top=.86, wspace=.24)
            for ax, operation in zip(axes, ['encode', 'decode']):
                for backend, policy, label, color in series:
                    selected = sorted([row for row in rows if (row['platform'], row['cache'], row['backend'], row['operation'])
                        == (platform, policy, backend, operation) and int(row['decoded_bytes']) > 0], key=lambda row: int(row['decoded_bytes']))
                    ax.plot([int(row['decoded_bytes']) for row in selected],
                        [float(row['warm_ns']) if row['status'] == 'measured' else float('nan') for row in selected],
                        color=color, label=label, linewidth=1.8, marker='D' if backend == 'five8' else 'o',
                        markersize=5 if backend == 'five8' else 2.8, linestyle='None' if backend == 'five8' else '-',
                        alpha=.96, zorder=5 if backend == 'five8' else 3)
                    measured = [row for row in selected if row['status'] == 'measured']
                    ax.errorbar([int(row['decoded_bytes']) for row in measured],
                        [float(row['warm_ns']) for row in measured],
                        yerr=[[float(row['warm_ns']) - float(row['process_min_ns']) for row in measured],
                              [float(row['process_max_ns']) - float(row['warm_ns']) for row in measured]],
                        fmt='none', ecolor=color, alpha=.3, elinewidth=.8, capsize=1.5)
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
            identity = next(row for row in rows if row['platform'] == platform)
            stamp = identity.get('measurement_date', '')
            source = identity.get('source_commit', '')[:12] or identity['source_fingerprint'][:12]
            figure.text(.5, .022, f'Public API · {stamp} · source {source} · warm medians; bars: observed process range\n'
                + ('V8 elapsed time' if platform == 'wasm' else 'Native process CPU time')
                + ' · Gaps represent explicit skips; no extrapolation', fontsize=9, ha='center', va='bottom')
            stem = {'m5': 'm5-public', 'gcp': 'gcp-public', 'wasm': 'wasm-public'}[platform]
            figure.savefig(destination / (stem + '.svg'), metadata={'Title': 'Base58 conversion time: ' + title, 'Date': None})
            plt.close(figure)
    (destination / 'public-tables.md').write_text(generated_tables, encoding='utf-8')
    print(destination)


if __name__ == '__main__':
    main()
