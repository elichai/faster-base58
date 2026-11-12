#!/usr/bin/env python3
"""Render the frozen 2026-11-12 Power-copy study into a new directory."""
import argparse
import csv
import os
from pathlib import Path
import tempfile

ROOT = Path(__file__).resolve().parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    rows = list(csv.DictReader((ROOT / 'data/power-copy-20261112-warm.csv').open()))
    assert len(rows) == 292
    candidate = next(r['candidate_commit'] for r in rows if r['cohort'] == 'primary')
    for row in rows:
        values = [float(row[k]) for k in ('ratio_min', 'ratio', 'ratio_max')]
        assert values == sorted(values) and values[0] > 0
    args.output.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix='base58-power-plot-') as cache:
        os.environ.setdefault('MPLCONFIGDIR', cache)
        import matplotlib
        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 10,
                            'axes.spines.top': False, 'axes.spines.right': False,
                            'svg.fonttype': 'path', 'svg.hashsalt': 'power-copy-20261112'})
        for host, title in [('m5', 'Apple M5 Pro'), ('gcp', 'AMD EPYC 9B45 · GCP')]:
            fig, axes = plt.subplots(2, 2, figsize=(12.5, 8.5), sharex=True)
            fig.subplots_adjust(left=.08, right=.985, bottom=.20, top=.85, hspace=.3, wspace=.2)
            for i, lto in enumerate(('fat', 'off')):
                for j, op in enumerate(('encode', 'decode')):
                    ax = axes[i, j]
                    ax.axhline(1, color='#6b7280', linestyle='--', linewidth=1)
                    subset = [r for r in rows if (r['host'], r['lto'], r['operation']) == (host, lto, op)]
                    for policy, color in [('default', '#1665ad'), ('cache', '#8057ae')]:
                        for label in ('head', 'ctrl'):
                            cells = sorted((r for r in subset if (r['policy'], r['candidate'], r['cohort'])
                                            == (policy, label, 'primary')), key=lambda r: int(r['decoded_bytes']))
                            x, y = [int(r['decoded_bytes']) for r in cells], [float(r['ratio']) for r in cells]
                            is_head = label == 'head'
                            ax.plot(x, y, color=color, marker='o' if is_head else 'x', markersize=3,
                                    linewidth=1.1 if is_head else .7, alpha=1 if is_head else .4,
                                    linestyle='-' if is_head else ':', label=policy.capitalize() + (' fix' if is_head else ' control'))
                            ax.errorbar(x, y, yerr=[[v - float(r['ratio_min']) for v, r in zip(y, cells)],
                                                   [float(r['ratio_max']) - v for v, r in zip(y, cells)]],
                                        fmt='none', color=color, alpha=.25 if is_head else .1, capsize=2, linewidth=.6)
                    repeat = [r for r in subset if (r['cohort'], r['candidate']) == ('indexed-repeat', 'head')]
                    if repeat:
                        ax.scatter([int(r['decoded_bytes']) for r in repeat], [float(r['ratio']) for r in repeat],
                                   marker='D', color='#c05216', s=25, label='Earlier indexed-candidate repeat', zorder=5)
                    ax.set(xscale='log', title=f"{op.capitalize()} · {'fat LTO' if lto == 'fat' else 'LTO off'}")
                    ax.set_xticks([32, 512, 2048, 32768, 1048576, 4194304])
                    ax.set_xticklabels(['32 B', '512 B', '2 KiB', '32 KiB', '1 MiB', '4 MiB'], rotation=25, ha='right')
                    ax.grid(axis='y', alpha=.18)
                    if j == 0:
                        ax.set_ylabel('Fix / published stack · lower is faster')
            fig.suptitle('Power-array copy removal · ' + title, y=.96, fontsize=16, fontweight='bold')
            fig.text(.5, .905, f'2026-11-12 · published stack f4b3f2fe2072 → measured fix {candidate[:12]}', ha='center')
            handles, labels = axes[0, 0].get_legend_handles_labels()
            fig.legend(handles, labels, loc='lower center', bbox_to_anchor=(.5, .09), ncol=3, frameon=False)
            fig.text(.5, .02, 'Public Auto · 5 processes × 3 copies × 3 batches · process CPU time\n'
                     'Bars: paired process ranges, not confidence intervals · dotted controls use the identical baseline file\n'
                     'Diamonds: earlier indexed form 43f79b820b18, separate 7-process × 5-copy repeat · baseline is not main',
                     ha='center', va='bottom', fontsize=9)
            fig.savefig(args.output / f'{host}-power-copy.svg', metadata={'Date': None})
            fig.savefig(args.output / f'{host}-power-copy.png', dpi=140)
            plt.close(fig)


if __name__ == '__main__':
    main()
