#!/usr/bin/env python3
"""Readable tables from every retained final-source compact cell/control."""
import argparse
import csv
import math
from pathlib import Path
import sys


def read(path):
    with path.open() as stream:
        return list(csv.DictReader(stream))


def size(value):
    value = int(value)
    if value >= 1048576:
        return f'{value // 1048576} MiB'
    if value >= 1024:
        return f'{value // 1024} KiB'
    return f'{value} B'


def latency(value):
    value = float(value)
    if not math.isfinite(value) or value <= 0:
        raise ValueError('Invalid finite timing')
    if value >= 1e6:
        return f'{value / 1e6:.3f} ms'
    if value >= 1000:
        return f'{value / 1000:.3f} µs'
    return f'{value:.3f} ns'


def pair(row):
    values = [float(row[name]) for name in ['alpha_median_cpu_ns','bravo_median_cpu_ns']]
    formatted = [latency(value) for value in values]
    winner = values.index(min(values))
    formatted[winner] = '**' + formatted[winner] + '**'
    return ' | '.join(formatted + [f"{float(row['cpu_ratio_median']):.6f}"])


def main():
    if sys.flags.optimize:
        raise SystemExit('Require Python optimize0')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data', type=Path, required=True)
    args = parser.parse_args()
    features = read(args.data/'feature-cells.csv')
    controls = read(args.data/'identical-controls.csv')
    if len(features) != 232 or len(controls) != 56:
        raise ValueError('Incomplete declared inventories')
    for cpu in ['generic','native']:
        for policy in ['default','cache']:
            selected = [row for row in features if row['cpu'] == cpu and row['policy'] == policy]
            by_key = {(row['operation'],int(row['bytes'])):row for row in selected}
            sizes = sorted({int(row['bytes']) for row in selected})
            if len(by_key) != 58 or len(sizes) != 29:
                raise ValueError('Wrong cohort domain')
            text = [f'# Final compact comparison: {cpu}, {policy}', '',
                'Ordinary Auto consumer on source 304dc. Latencies are process CPU nanoseconds per conversion, formatted by magnitude. '
                'Each value is a median of five fresh-process warm medians. Each process has three 100 ms warm batches. '
                'Ratio is the median of five paired compact/wide ratios and can differ from the displayed-median quotient. '
                'Bold marks the lower observed median, without a significance claim.', '',
                '| Payload | Wide encode | Compact encode | Ratio | Wide decode | Compact decode | Ratio |',
                '|---|---:|---:|---:|---:|---:|---:|']
            for n in sizes:
                text.append('| '+size(n)+' | '+pair(by_key['encode',n])+' | '+pair(by_key['decode',n])+' |')
            text += ['', 'All five paired ratios remain in [feature-cells.csv](feature-cells.csv); '
                     '[identical-content controls](identical-controls.md) and [all pairs](all-pairs.csv) are retained separately. '
                     'Tiny conversions include call/context costs and do not execute IFMA.']
            with (args.data/(cpu+'-'+policy+'.md')).open('x') as stream:
                stream.write('\n'.join(text)+'\n')
    text = ['# All identical-content control cells', '',
        'Both labels install the same wide artifact into the same global inode. Every pair is retained. '
        'Median compact-label/wide-label ratios range 0.993936–1.018033; individual pairs range 0.812408–1.114856. '
        'These are observed controls, not a guarantee of sub-percent stability.', '',
        '| CPU | Policy | Observer backend | Direction | Payload | Alpha | Bravo | Median ratio | All five ratios |',
        '|---|---|---|---|---:|---:|---:|---:|---|']
    for row in sorted(controls,key=lambda row:(row['cpu'],row['policy'],row['backend'],row['operation'],int(row['bytes']))):
        text.append('| '+' | '.join([row['cpu'],row['policy'],row['backend'],row['operation'],size(row['bytes']),
            latency(row['alpha_median_cpu_ns']),latency(row['bravo_median_cpu_ns']),f"{float(row['cpu_ratio_median']):.6f}",
            ', '.join(f'{float(value):.6f}' for value in row['all_five_cpu_ratios'].split(';'))])+' |')
    with (args.data/'identical-controls.md').open('x') as stream:
        stream.write('\n'.join(text)+'\n')


if __name__ == '__main__':
    main()
