#!/usr/bin/env python3
"""Reject public integration tests omitted from the explicit CI test lists."""
import argparse
from pathlib import Path
import re
import shlex
import tomllib

ROOT = Path(__file__).resolve().parents[1]
INTERNAL_CFG = {
    'matrix_cold': '#![cfg(feature = "__internals")]',
    'resource_preflight': '#![cfg(feature = "__internals")]',
    'miri_intrinsics': '#![cfg(miri)]',
}


def public_targets(root):
    manifest = tomllib.loads((root / 'Cargo.toml').read_text())
    required = {target['name'] for target in manifest.get('test', [])
                if target.get('required-features')}
    files = {path.stem: path for path in (root / 'tests').glob('*.rs')}
    for name, marker in INTERNAL_CFG.items():
        if name not in files or marker not in files[name].read_text():
            raise ValueError('internal test classification changed: ' + name)
    return set(files) - required - set(INTERNAL_CFG)


def check_lists(workflow, expected):
    lists = []
    for line in workflow.splitlines():
        if re.search(r'run: cargo \+\S+ test --locked --test api\b', line):
            words = shlex.split(line.split('run: ', 1)[1])
            names = [words[index + 1] for index, word in enumerate(words) if word == '--test']
            if len(names) != len(set(names)) or set(names) != expected:
                raise ValueError('public test list mismatch: ' + repr(names) + '; expected ' + repr(sorted(expected)))
            lists.append(names)
    if len(lists) != 3:
        raise ValueError('expected the stable, MSRV std, and MSRV cache explicit test lists')
    return lists


def self_test():
    good = '\n'.join('      - run: cargo +' + tool + ' test --locked --test api --test owned_output'
                     for tool in ['stable', '1.95', '1.95'])
    expected = {'api', 'owned_output'}
    if len(check_lists(good, expected)) != 3:
        raise AssertionError('valid test lists rejected')
    for bad in [good.replace(' --test owned_output', '', 1),
                good.replace('--test owned_output', '--test api', 1),
                '\n'.join(good.splitlines()[:2])]:
        try:
            check_lists(bad, expected)
        except ValueError:
            continue
        raise AssertionError('missing/duplicated/unclassified test list accepted')
    print('check_ci_tests self-test: pass')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--source', type=Path, default=ROOT)
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    targets = public_targets(args.source)
    check_lists((args.source / '.github/workflows/verification.yml').read_text(), targets)
    print('Public integration test lists cover: ' + ', '.join(sorted(targets)))


if __name__ == '__main__':
    main()
