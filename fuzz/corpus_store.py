#!/usr/bin/env python3
"""Validate and add fuzz inputs to a content-addressed corpus."""

import argparse
import hashlib
import json
import re
import stat
from pathlib import Path

TARGETS = ("encode_all", "decode_all", "encode_full_width")
MAX_BYTES = 89_502
MAX_ITEMS = 50_000
MAX_TOTAL_BYTES = 512 * 1024 * 1024
SHA256 = re.compile(r"[0-9a-f]{64}\Z")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def _regular(path):
    return stat.S_ISREG(path.lstat().st_mode)


def _target_directories(root):
    if root.is_symlink() or not root.is_dir():
        raise ValueError(f"corpus root is not a directory: {root}")
    for entry in sorted(root.iterdir()):
        if entry.name not in TARGETS or entry.is_symlink() or not entry.is_dir():
            raise ValueError(f"unsupported corpus target: {entry}")
        yield entry


def _inputs(directory, max_bytes, canonical):
    for entry in sorted(directory.iterdir()):
        if not _regular(entry):
            raise ValueError(f"corpus item is not a regular file: {entry}")
        size = entry.stat().st_size
        if size > max_bytes:
            raise ValueError(f"corpus item exceeds {max_bytes} bytes: {entry}")
        data = entry.read_bytes()
        content_digest = digest(data)
        if canonical and entry.name != content_digest:
            raise ValueError(f"corpus item name is not its SHA-256: {entry}")
        yield entry, data, content_digest


def validate(root, max_bytes=MAX_BYTES, max_items=MAX_ITEMS, max_total_bytes=MAX_TOTAL_BYTES):
    """Reject a replay corpus with a bad target, file type, size, or name."""
    items = 0
    total_bytes = 0
    for directory in _target_directories(root):
        for _entry, data, _digest in _inputs(directory, max_bytes, canonical=True):
            items += 1
            total_bytes += len(data)
            if items > max_items:
                raise ValueError(f"corpus exceeds {max_items} item limit")
            if total_bytes > max_total_bytes:
                raise ValueError(f"corpus exceeds {max_total_bytes} byte limit")


def stage(incoming, corpus, max_bytes=MAX_BYTES):
    """Add valid artifact files under their SHA-256 names without replacement."""
    if corpus.is_symlink():
        raise ValueError(f"corpus root is not a directory: {corpus}")
    corpus.mkdir(parents=True, exist_ok=True)
    if not corpus.is_dir():
        raise ValueError(f"corpus root is not a directory: {corpus}")
    added = {target: [] for target in TARGETS}
    for directory in _target_directories(incoming):
        destination = corpus / directory.name
        destination.mkdir(parents=True, exist_ok=True)
        if destination.is_symlink() or not destination.is_dir():
            raise ValueError(f"corpus target is not a directory: {destination}")
        for _entry, data, content_digest in _inputs(directory, max_bytes, canonical=False):
            output = destination / content_digest
            if output.exists() or output.is_symlink():
                if not _regular(output) or output.read_bytes() != data:
                    raise ValueError(f"existing corpus item differs: {output}")
                continue
            with output.open("xb") as file:
                file.write(data)
            added[directory.name].append(content_digest)
    validate(corpus, max_bytes=max_bytes)
    return {target: values for target, values in added.items() if values}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    stage_parser = commands.add_parser("stage", help="add artifact inputs to a corpus")
    stage_parser.add_argument("incoming", type=Path)
    stage_parser.add_argument("corpus", type=Path)
    validate_parser = commands.add_parser("validate", help="validate a replay corpus")
    validate_parser.add_argument("corpus", type=Path)
    args = parser.parse_args()
    if args.command == "stage":
        print(json.dumps(stage(args.incoming, args.corpus), sort_keys=True))
    else:
        validate(args.corpus)


if __name__ == "__main__":
    main()
