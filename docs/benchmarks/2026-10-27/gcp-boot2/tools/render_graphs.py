#!/usr/bin/env python3
"""Render dated four-panel internal-backend research figures; never benchmark."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import tempfile

BACKENDS = ['auto', 'no-alloc', 'batched', 'karatsuba', 'toom3', 'matrix', 'matrix-portable',
            'ntt', 'ntt-cached', 'ntt-portable', 'ntt-same-owned', 'ntt-same-cached',
            'matrix-forced', 'malachite', 'bs58', 'five8']
TITLES = {'m5':'Apple M5 Pro · compiler-default target',
          'wasm':'WebAssembly / V8 on Apple M5 Pro · SIMD128 off',
          'gcp-generic':'GCP AMD EPYC 9B45 / c4d-highcpu-4 · generic target',
          'gcp-native':'GCP AMD EPYC 9B45 / c4d-highcpu-4 · target-cpu=native'}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def duration(value):
    scale, unit = (1e9,'s') if value>=1e9 else (1e6,'ms') if value>=1e6 else (1e3,'µs') if value>=1e3 else (1,'ns')
    return f'{value / scale:.3g} {unit}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--preview', action='store_true', help='private PNG previews, excluded from the export manifest')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    with args.data.open(newline='') as stream:
        rows = list(csv.DictReader(stream))
    platforms = [name for name in TITLES if any(row['platform']==name for row in rows)]
    values = [row for row in rows if row['status']=='measured' and int(row['decoded_bytes'])>0]
    with tempfile.TemporaryDirectory(prefix='base58-research-plots-') as cache:
        os.environ.setdefault('MPLCONFIGDIR', cache)
        os.environ.setdefault('XDG_CACHE_HOME', cache)
        import matplotlib
        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        from matplotlib.ticker import FuncFormatter
        plt.rcParams.update({'font.family':'DejaVu Sans','font.size':10,'axes.spines.top':False,
            'axes.spines.right':False,'svg.fonttype':'none','svg.hashsalt':'faster-base58-backends-2026-10-27'})
        colors = plt.get_cmap('tab20').colors
        outputs = {}
        for platform in platforms:
            fig, axes = plt.subplots(2,2,figsize=(16,12))
            fig.subplots_adjust(left=.075,right=.99,bottom=.20,top=.89,wspace=.22,hspace=.32)
            for row_index, operation in enumerate(['encode','decode']):
                for column, policy in enumerate(['default','cache']):
                    ax = axes[row_index,column]
                    for index, backend in enumerate(BACKENDS):
                        selected = sorted([row for row in values if (row['platform'],row['cache'],row['operation'],row['backend'])
                            == (platform,policy,operation,backend)], key=lambda row:int(row['decoded_bytes']))
                        assert selected, (platform,policy,operation,backend)
                        ax.plot([int(row['decoded_bytes']) for row in selected], [float(row['warm_ns']) for row in selected],
                            label=backend,color=colors[index],linewidth=2 if backend=='auto' else 1.15,
                            marker='D' if backend=='five8' else 'o',markersize=5 if backend=='five8' else 2,
                            linestyle='None' if backend=='five8' else ':' if 'same-' in backend else '--' if 'portable' in backend else '-')
                    ax.set(xscale='log',yscale='log',title=operation.capitalize()+' · '+policy,
                        xlabel='Decoded payload size',ylabel='Time per conversion · lower is faster')
                    ax.set_xticks([1,32,64,4096,65536,1048576,4194304])
                    ax.set_xticklabels(['1 B','32 B','64 B','4 KiB','64 KiB','1 MiB','4 MiB'],rotation=22,ha='right')
                    ax.yaxis.set_major_formatter(FuncFormatter(lambda value,_:duration(value)))
                    ax.grid(which='major',alpha=.18,linewidth=.7)
                    ax.grid(which='minor',alpha=.05,linewidth=.4)
            fig.suptitle('Base58 · all backends and controls\n'+TITLES[platform],fontsize=16,fontweight='bold',y=.97)
            handles, labels = axes[0,0].get_legend_handles_labels()
            assert labels == BACKENDS
            fig.legend(handles,labels,loc='lower center',bbox_to_anchor=(.5,.055),ncol=6,frameon=False,fontsize=9)
            clock = 'V8 elapsed time' if platform=='wasm' else 'Process CPU time'
            scope = 'separate linked backend modules' if platform=='wasm' else 'one internal all-backend worker'
            fig.text(.5,.012,clock+' · '+scope+' · 3 fresh processes × 3 warm batches\n'
                'Wrappers can share fallbacks; route labels and temporal Auto controls remain separate in the CSV. '
                'Quadratic rows stop at 64 KiB; five8 supports 32/64 B; no extrapolation.',ha='center',va='bottom',fontsize=9)
            name = platform+'-backends.svg'
            fig.savefig(args.output/name,metadata={'Title':'Base58 all-backend comparison: '+TITLES[platform],'Date':None})
            if args.preview:
                fig.savefig(args.output/(platform+'-backends.png'),dpi=110)
            plt.close(fig)
            outputs[name] = sha(args.output/name)
        with (args.output/'metadata.json').open('x') as stream:
            json.dump(dict(status='rendered',platforms=platforms,input_sha256=sha(args.data),
                renderer_sha256=sha(Path(__file__)),matplotlib=matplotlib.__version__,outputs=outputs,
                scope='Internal native worker or the original per-backend Wasm module cohort only; ordinary public figures are separate.',
                controls='Every registered backend/control except empty scaffold; missing/capped rows are not extrapolated.',
                estimator='Median of9 warm batches from3 fresh processes; native CPU and Wasm elapsed clocks stay separate.'),stream,indent=2)
            stream.write('\n')
    print('Rendered',len(platforms),'four-panel full-backend figure sets')


if __name__=='__main__':
    main()
