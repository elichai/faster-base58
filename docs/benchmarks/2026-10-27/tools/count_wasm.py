#!/usr/bin/env python3
"""Collect fresh-instance Wasm requested layouts, never latency or linked size."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import random
import shutil
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
from process_supervisor import install_signal_handlers, owned_process


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def skip(semantics, operation, size, encoded):
    support = semantics['measurement_support']
    if size not in support.get('decoded_bytes', [size]):
        return 'fixed-width domain'
    if size > support.get('max_decoded_bytes', 4194304):
        return 'quadratic measurement cap'
    domain = support.get(operation, {})
    if not domain.get('min_decoded_bytes', 0) <= size <= domain.get('max_decoded_bytes', 4194304):
        return 'outside forced Matrix decoded-byte domain'
    if not domain.get('min_encoded_chars', 0) <= encoded <= domain.get('max_encoded_chars', 8388608):
        return 'outside forced Matrix text domain'
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['modules', 'fixtures', 'manifest', 'output']:
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--backends', help='comma-separated subset; defaults to every both-direction artifact')
    parser.add_argument('--sizes', help='comma-separated subset of the independent fixture manifest')
    args = parser.parse_args()
    install_signal_handlers()
    args.output.mkdir(parents=True, exist_ok=False)
    build = json.loads((args.modules / 'metadata.json').read_text())
    assert build['count_allocations'] is True
    artifacts = {}
    with (args.modules / 'sizes.csv').open() as stream:
        for row in csv.DictReader(stream):
            assert (row['direction'], row['kind']) == ('both', 'cdylib')
            path = args.modules / f"{row['backend']}-both-cdylib.wasm.stripped"
            assert sha(path) == row['stripped_sha256']
            artifacts[row['backend']] = row['stripped_sha256']
    selected = args.backends.split(',') if args.backends else list(artifacts)
    assert len(selected) == len(set(selected)) and set(selected) <= artifacts.keys()
    fixtures = json.loads(args.manifest.read_text())['fixtures']
    sizes = set(map(int, args.sizes.split(','))) if args.sizes else {row['decoded_bytes'] for row in fixtures}
    assert sizes <= {row['decoded_bytes'] for row in fixtures}
    jobs, skipped = [], []
    for backend in selected:
        for fixture in fixtures:
            size, encoded = fixture['decoded_bytes'], fixture['encoded_chars']
            if size not in sizes:
                continue
            for operation in ['encode', 'decode']:
                reason = skip(build['backend_semantics'][backend], operation, size, encoded)
                if reason:
                    skipped.append(dict(backend=backend, operation=operation, decoded_bytes=size, reason=reason))
                else:
                    jobs.append((backend, operation, size))
    random.Random(20261027).shuffle(jobs)
    metadata = dict(status='running', scope='Requested layouts only; fresh process/instance for each cold+warm pair',
        build_metadata_sha256=sha(args.modules/'metadata.json'), fixture_manifest_sha256=sha(args.manifest),
        helper_sha256=sha(ROOT/'scripts/wasm_allocations.mjs'), worker_sha256=sha(HERE/'count_wasm.mjs'),
        source_sha256=build['source_sha256'], cache=build['cache'], target='wasm32-unknown-unknown',
        script_sha256=sha(Path(__file__)), modules=artifacts, expected_cases=len(jobs), skipped_cases=len(skipped),
        node=subprocess.check_output(['node','--version'],text=True).strip(), jobs=jobs,
        note='Self-check ABI on separate disposable instances before this collector; actual measurement instances stay fresh. '
             'Requested layouts exclude allocator headers/slack and realloc-copy overlap; pages are not live heap or RSS.')
    initial_metadata = json.dumps(metadata,indent=2)+'\n'
    (args.output/'metadata.json').write_text(initial_metadata)
    with (args.output/'skipped.csv').open('x',newline='') as stream:
        writer=csv.DictWriter(stream,fieldnames=['backend','operation','decoded_bytes','reason'])
        writer.writeheader()
        writer.writerows(skipped)
    with (args.output/'raw.jsonl').open('x') as stream:
        for index, (backend,operation,size) in enumerate(jobs,1):
            assert shutil.disk_usage(args.output).free >= 2*1024**3, '2 GiB disk floor reached'
            command=['node',str(HERE/'count_wasm.mjs'),str(args.modules.resolve()),str(args.fixtures.resolve()),
                str(args.manifest.resolve()),backend,operation,str(size),artifacts[backend]]
            with owned_process(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True) as child:
                stdout,stderr=child.communicate(timeout=600)
                assert child.returncode==0,stderr
            value=json.loads(stdout)
            assert value['module_sha256']==artifacts[backend]
            stream.write(json.dumps(dict(command=command,result=value))+'\n')
            stream.flush()
            print(index,'/',len(jobs),backend,operation,size,flush=True)
    assert sha(args.modules/'metadata.json') == metadata['build_metadata_sha256'], 'Build metadata changed during collection'
    assert sha(args.manifest) == metadata['fixture_manifest_sha256'], 'Fixture manifest changed during collection'
    assert sha(ROOT/'scripts/wasm_allocations.mjs') == metadata['helper_sha256'], 'Counter helper changed during collection'
    assert sha(HERE/'count_wasm.mjs') == metadata['worker_sha256'], 'Counter worker changed during collection'
    assert sha(Path(__file__)) == metadata['script_sha256'], 'Collector changed during collection'
    for backend in selected:
        assert sha(args.modules/f'{backend}-both-cdylib.wasm.stripped') == artifacts[backend], backend
    assert (args.output/'metadata.json').read_text() == initial_metadata, 'Owned output metadata changed during collection'
    metadata.update(status='complete',completed_cases=len(jobs),raw_sha256=sha(args.output/'raw.jsonl'))
    (args.output/'metadata.json').write_text(json.dumps(metadata,indent=2)+'\n')


if __name__=='__main__':
    main()
