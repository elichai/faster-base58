#!/usr/bin/env python3
"""Validate retained documentation links and SVG structure; never run benchmarks.

This check does not verify measurements or regenerate table values or pixels.
Use render_current.py or render_backends.py with external CSVs to update figures.
"""
import argparse
import json
from pathlib import Path

from current import check


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', required=True,
                        help='check documentation links, selected figures and SVG syntax')
    parser.parse_args()
    try:
        result = check(Path(__file__).resolve().parents[2])
    except (OSError, ValueError) as error:
        parser.exit(1, f'Documentation check failed: {error}\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
