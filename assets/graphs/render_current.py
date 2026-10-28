#!/usr/bin/env python3
"""Regenerate current public figures from committed CSVs; never run benchmarks.

Requires the separately pinned Matplotlib environment. CI checks committed SVG
hashes, numeric tables and input provenance without installing plotting packages.
"""
import argparse
import json
import os
from pathlib import Path
import tempfile

from current import DATA, digest, read, tables


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='new empty destination; never overwrite committed files')
    args = parser.parse_args()
    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=False)
    root = Path(__file__).resolve().parents[2]
    rows = read(root / DATA / 'public.csv')
    measured = [row for row in rows if row['status'] == 'measured']
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
                  ('five8', 'default', 'five8 (32/64 B)', '#16866b')]
        plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 10, 'axes.titlesize': 12,
            'axes.spines.top': False, 'axes.spines.right': False, 'svg.fonttype': 'none',
            'svg.hashsalt': 'faster-base58-public-2026-10-27'})
        for platform, title in [('m5', 'Apple M5 Pro'), ('wasm', 'WebAssembly on Apple M5 Pro (Node/V8)')]:
            figure, axes = plt.subplots(1, 2, figsize=(13, 6.4))
            figure.subplots_adjust(left=.075, right=.99, bottom=.25, top=.86, wspace=.24)
            for ax, operation in zip(axes, ['encode', 'decode']):
                for backend, policy, label, color in series:
                    selected = sorted([row for row in measured if (row['platform'], row['cache'], row['backend'], row['operation'])
                        == (platform, policy, backend, operation) and int(row['decoded_bytes']) > 0], key=lambda row: int(row['decoded_bytes']))
                    ax.plot([int(row['decoded_bytes']) for row in selected], [float(row['warm_ns']) for row in selected],
                        color=color, label=label, linewidth=1.8, marker='D' if backend == 'five8' else 'o',
                        markersize=5 if backend == 'five8' else 2.8, linestyle='None' if backend == 'five8' else '-',
                        alpha=.96, zorder=5 if backend == 'five8' else 3)
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
            figure.text(.5, .022, 'Public API · default target flags · 3 fresh processes × 3 batches\n'
                + ('Native process CPU time' if platform == 'm5' else 'V8 elapsed time; SIMD128 off')
                + ' · Quadratic measurements stop at 64 KiB; no extrapolation', fontsize=9, ha='center', va='bottom')
            stem = 'm5-public' if platform == 'm5' else 'wasm-public'
            figure.savefig(destination / (stem + '.svg'), metadata={'Title': 'Base58 conversion time: ' + title, 'Date': None})
            plt.close(figure)
        version = matplotlib.__version__
    (destination / 'public-tables.md').write_text(tables(rows, read(root / DATA / 'figure-linked-sizes.csv')))
    (destination / 'render-receipt.json').write_text(json.dumps(dict(status='rendered', matplotlib=version,
        renderer_sha256=digest(Path(__file__)), tables_helper_sha256=digest(Path(__file__).with_name('current.py')),
        inputs={str(DATA / name): digest(root / DATA / name) for name in ['public.csv', 'figure-linked-sizes.csv']},
        outputs={path.name: digest(path) for path in destination.iterdir() if path.is_file()},
        note='Portable renderer preserves the measured values and appearance. The original v3 SVG hashes are retained '
             'in public-graphs.json; this command does not silently replace their provenance.'), indent=2) + '\n')
    print(destination)


if __name__ == '__main__':
    main()
