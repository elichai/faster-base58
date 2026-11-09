#!/usr/bin/env python3
"""Validate content-addressed fuzz corpora and write inputs into them."""

import argparse
import hashlib
import json
import re
import shutil
import stat
import tempfile
from pathlib import Path

TARGETS = ("encode_all", "decode_all", "encode_full_width", "encode_radix_full_width")
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


def target_digests(root, max_bytes=MAX_BYTES):
    """Map each target directory under root to the SHA-256 digests of its files."""
    return {
        target: {content_digest for _entry, _data, content_digest
                 in _inputs(root / target, max_bytes, canonical=False)}
        for target in TARGETS if (root / target).exists()
    }


def _add(incoming, corpus, max_bytes, exclude):
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
        skip = exclude.get(directory.name, ())
        for _entry, data, content_digest in _inputs(directory, max_bytes, canonical=False):
            if content_digest in skip:
                continue
            output = destination / content_digest
            if output.exists() or output.is_symlink():
                if not _regular(output) or output.read_bytes() != data:
                    raise ValueError(f"existing corpus item differs: {output}")
                continue
            with output.open("xb") as file:
                file.write(data)
            added[directory.name].append(content_digest)
    return {target: values for target, values in added.items() if values}


def stage(incoming, corpus, max_bytes=MAX_BYTES):
    """Add valid artifact files under their SHA-256 names without replacement."""
    added = _add(incoming, corpus, max_bytes, {})
    validate(corpus, max_bytes=max_bytes)
    return added


def replace(incoming, corpus, exclude=None, max_bytes=MAX_BYTES):
    """Replace corpus with the union of the incoming roots.

    Inputs whose bytes occur under ``exclude/<target>`` are skipped. An existing
    corpus must validate before it is removed.
    """
    if corpus.is_symlink():
        raise ValueError(f"corpus root is not a directory: {corpus}")
    if corpus.exists():
        validate(corpus, max_bytes=max_bytes)
    excluded = target_digests(exclude, max_bytes) if exclude else {}
    corpus.parent.mkdir(parents=True, exist_ok=True)
    fresh = Path(tempfile.mkdtemp(prefix=f".{corpus.name}-", dir=corpus.parent))
    try:
        for target in TARGETS:
            (fresh / target).mkdir()
        for root in incoming:
            _add(root, fresh, max_bytes, excluded)
        validate(fresh, max_bytes=max_bytes)
    except BaseException:
        shutil.rmtree(fresh)
        raise
    if corpus.exists():
        shutil.rmtree(corpus)
    fresh.rename(corpus)
    return {target: sum(1 for _ in (corpus / target).iterdir()) for target in TARGETS}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    stage_parser = commands.add_parser("stage", help="add artifact inputs to a corpus")
    stage_parser.add_argument("incoming", type=Path)
    stage_parser.add_argument("corpus", type=Path)
    replace_parser = commands.add_parser("replace", help="replace a corpus with the union of inputs")
    replace_parser.add_argument("incoming", type=Path, nargs="+")
    replace_parser.add_argument("corpus", type=Path)
    replace_parser.add_argument("--exclude", type=Path, metavar="ROOT",
                                help="skip inputs whose bytes occur under ROOT/<target>")
    validate_parser = commands.add_parser("validate", help="validate a replay corpus")
    validate_parser.add_argument("corpus", type=Path)
    args = parser.parse_args()
    if args.command == "stage":
        print(json.dumps(stage(args.incoming, args.corpus), sort_keys=True))
    elif args.command == "replace":
        counts = replace(args.incoming, args.corpus, exclude=args.exclude)
        print(json.dumps(counts, sort_keys=True))
    else:
        validate(args.corpus)


if __name__ == "__main__":
    main()
