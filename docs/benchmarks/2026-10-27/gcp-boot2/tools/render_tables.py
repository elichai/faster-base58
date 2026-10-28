#!/usr/bin/env python3
"""Render readable full-grid backend tables and optional SVGs from retained CSVs."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import tempfile

SIZES = [0,1,8,16,21,22,23,31,32,33,63,64,65,128,256,512,1024,2048,4096,8192,
         16384,32768,65536,131072,262144,524288,1048576,2097152,4194304]
GROUPS = [('algorithms', ['auto','no-alloc','karatsuba','toom3','matrix','ntt-cached','malachite','bs58','five8']),
          ('kernel-controls', ['auto','batched','ntt','ntt-cached','ntt-portable','ntt-same-owned','ntt-same-cached','matrix-forced','matrix-portable'])]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def duration(value):
    scale, unit = (1e9,'s') if value>=1e9 else (1e6,'ms') if value>=1e6 else (1e3,'µs') if value>=1e3 else (1,'ns')
    return f'{value / scale:.3g} {unit}'


def payload(value):
    return f'{value // 1048576} MiB' if value>=1048576 else f'{value // 1024} KiB' if value>=1024 else f'{value} B'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--plots', action='store_true')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    with args.data.open(newline='') as stream:
        rows = list(csv.DictReader(stream))
    keyed = {(row['platform'],row['cache'],row['backend'],row['operation'],int(row['decoded_bytes'])):row for row in rows}
    assert len(keyed) == len(rows)
    names = []
    for platform in ['gcp-generic','gcp-native']:
        for policy in ['default','cache']:
            for group, backends in GROUPS:
                text = [f'# {platform} / {policy} / {group}', '',
                    'Frozen `4e5ac883…` backend comparison on AMD EPYC 9B45 / c4d-highcpu-4. '
                    'Each CPU/cache cohort uses one immutable all-backend `__internals` worker. '
                    'Generic uses default target flags; native uses actual `target-cpu=native`. '
                    'These are separate from final-source304dc ordinary compact comparisons.', '',
                    'Time is the median of nine warm batch observations from three fresh processes × three batches. '
                    'Time uses process CPU. Bold marks the lowest observed median within this table, not statistical significance.', '',
                    '`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. '
                    'No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.', '']
                for operation in ['encode','decode']:
                    text += ['## ' + operation.capitalize(), '', '| Payload | ' + ' | '.join(backends) + ' |',
                             '|---|' + '---:|' * len(backends)]
                    for size in SIZES:
                        values = [keyed.get((platform,policy,backend,operation,size)) for backend in backends]
                        measured = [float(row['warm_ns']) for row in values if row and row['status']=='measured']
                        lowest = min(measured) if measured else None
                        formatted = []
                        for row in values:
                            if row is None:
                                formatted.append('pending')
                            elif row['status']=='skipped':
                                formatted.append('—')
                            else:
                                value = float(row['warm_ns'])
                                formatted.append('**' + duration(value) + '**' if value==lowest else duration(value))
                        text.append('| ' + payload(size) + ' | ' + ' | '.join(formatted) + ' |')
                    text += ['']
                name = f'{platform}-{policy}-{group}.md'
                (args.output / name).write_text('\n'.join(text))
                names.append(name)
    matplotlib_version = None
    if args.plots:
        with tempfile.TemporaryDirectory(prefix='base58-backend-plots-') as cache:
            os.environ.setdefault('MPLCONFIGDIR',cache)
            os.environ.setdefault('XDG_CACHE_HOME',cache)
            import matplotlib
            matplotlib.use('Agg')
            import matplotlib.pyplot as plt
            from matplotlib.ticker import FuncFormatter
            matplotlib_version = matplotlib.__version__
            plt.rcParams.update({'font.family':'DejaVu Sans','font.size':9,'axes.spines.top':False,
                'axes.spines.right':False,'svg.fonttype':'none','svg.hashsalt':'faster-base58-backends-2026-10-27'})
            graph_backends = ['auto','no-alloc','batched','karatsuba','toom3','matrix','matrix-portable',
                              'ntt','ntt-cached','ntt-portable','malachite','bs58','five8']
            colors = plt.get_cmap('tab20').colors
            for platform in ['gcp-generic','gcp-native']:
                for policy in ['default','cache']:
                    figure, axes = plt.subplots(1,2,figsize=(14,7.4))
                    figure.subplots_adjust(left=.075,right=.985,bottom=.30,top=.85,wspace=.24)
                    for ax, operation in zip(axes,['encode','decode']):
                        for index, backend in enumerate(graph_backends):
                            selected = [keyed.get((platform,policy,backend,operation,size)) for size in SIZES if size>0]
                            selected = [row for row in selected if row is not None and row['status']=='measured']
                            if not selected:
                                continue
                            ax.plot([int(row['decoded_bytes']) for row in selected],[float(row['warm_ns']) for row in selected],
                                label=backend,color=colors[index],linewidth=1.7 if backend=='auto' else 1.2,
                                marker='D' if backend=='five8' else 'o',markersize=5 if backend=='five8' else 2,
                                linestyle='None' if backend=='five8' else '--' if 'portable' in backend else '-')
                        ax.set(xscale='log',yscale='log',title=operation.capitalize(),xlabel='Decoded payload size',ylabel='Time per conversion')
                        ax.set_xticks([1,32,64,512,4096,65536,1048576,4194304])
                        ax.set_xticklabels(['1 B','32 B','64 B','512 B','4 KiB','64 KiB','1 MiB','4 MiB'],rotation=28,ha='right')
                        ax.yaxis.set_major_formatter(FuncFormatter(lambda value,_:duration(value)))
                        ax.grid(which='major',alpha=.18,linewidth=.7)
                    title = 'AMD EPYC 9B45 / c4d-highcpu-4 · '+platform
                    figure.suptitle('Base58 backend comparison · ' + title + ' · ' + policy,fontsize=14,fontweight='bold',y=.97)
                    handles, labels = axes[0].get_legend_handles_labels()
                    figure.legend(handles,labels,loc='lower center',bbox_to_anchor=(.5,.075),ncol=5,frameon=False)
                    figure.text(.5,.025,'Frozen4e5 backend survey · 3 fresh processes × 3 warm batches · '
                        + 'Process CPU' + '\nWrappers can share fallbacks; '
                        'quadratic rows stop at 64 KiB; no extrapolation. See tables and route labels.',ha='center',va='bottom',fontsize=9)
                    name = platform + '-' + policy + '-backends.svg'
                    figure.savefig(args.output/name,metadata={'Title':'Base58 backend comparison: '+title+' '+policy,'Date':None})
                    plt.close(figure)
                    names.append(name)
    with (args.output/'metadata.json').open('x') as stream:
        json.dump(dict(status='rendered from retained CSV; measurement completeness is in the input metadata',
            input_sha256=sha(args.data),script_sha256=sha(Path(__file__)),matplotlib=matplotlib_version,
            outputs={name:sha(args.output/name) for name in names},
            estimator='Median of9 batches; selected ordinary public figures use median of3 process medians instead'),stream,indent=2)
        stream.write('\n')
    print('Rendered',len(names),'backend table/figure files')


if __name__=='__main__':
    main()
