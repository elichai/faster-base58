#!/usr/bin/env python3
"""Write immutable JSON provenance for one fuzz campaign."""

import argparse
import hashlib
import json
import re
from pathlib import Path

KEY = re.compile(r"[a-z][a-z0-9_]*\Z")


def write(output, values):
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("x") as file:
        json.dump(values, file, sort_keys=True)
        file.write("\n")


def file_digest(path):
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(64 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def add_digest(values, key, path):
    digest_key = key + "_sha256"
    if not KEY.fullmatch(key) or digest_key in values:
        raise ValueError(f"invalid or duplicate digest field: {key}")
    values[digest_key] = file_digest(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--field", action="append", default=[], metavar="KEY=VALUE")
    parser.add_argument("--digest", action="append", default=[], metavar="KEY=PATH")
    args = parser.parse_args()
    values = {}
    for field in args.field:
        key, separator, value = field.partition("=")
        if not separator or not KEY.fullmatch(key) or key in values:
            raise ValueError(f"invalid or duplicate provenance field: {field}")
        values[key] = value
    for field in args.digest:
        key, separator, path = field.partition("=")
        if not separator:
            raise ValueError(f"invalid digest field: {field}")
        add_digest(values, key, Path(path))
    write(args.output, values)


if __name__ == "__main__":
    main()
