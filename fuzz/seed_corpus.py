#!/usr/bin/env python3
"""Write deterministic corpus seeds; no packages or network access required."""

import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "corpus"
ENCODE_MAX_BYTES = 64 * 1024 + 1
# max_encoded_len(ENCODE_MAX_BYTES), matching the Rust decode target and CI.
DECODE_MAX_BYTES = 89_502
SMALL_MAX_BYTES = 2048
LENGTHS = [
    0, 1, 6, 7, 8, 9, 10, 11, 21, 22, 23, 31, 32, 33, 63, 64, 65,
    511, 512, 513, 767, 768, 769, 1279, 1280, 1281, 1503, 1504, 1505,
    3071, 3072, 3073, 3583, 3584, 3585, 4095, 4096, 4097, 5119, 5120,
    5121, 5631, 5632, 5633, 8191, 8192, 8193, 16383, 16384, 16385,
    32767, 32768, 32769, ENCODE_MAX_BYTES - 2, ENCODE_MAX_BYTES - 1, ENCODE_MAX_BYTES,
]
DECODE_CARRY_LENGTHS = [
    # Keep seeds from the earlier campaign, including its old 89,493-byte cap.
    44_760, 44_761, 89_492, 89_493,
    # All-z inputs cross 8192 u64 limbs at 89,500 characters, then reach the new cap.
    89_499, 89_500, DECODE_MAX_BYTES - 1, DECODE_MAX_BYTES,
]
UNICODE_LENGTHS = [23, 64, 769, 8193]
INVALID_UNICODE = [("two-byte", "é"), ("three-byte", "中"), ("four-byte", "🦀")]
RADIX_LENGTHS = [33, 64, 65, 127, 128, 129, 255, 256, 257, 511, 512, 513,
                 8191, 8192, 8193, ENCODE_MAX_BYTES]


def write_seed(target, name, data):
    path = ROOT / target / name
    if path.exists():
        if path.read_bytes() != data:
            raise ValueError(f"Existing seed has manual changes: {path}")
    else:
        path.write_bytes(data)


def main():
    for target in ["encode_all", "decode_all", "encode_small", "decode_small", "encode_full_width", "encode_radix_full_width"]:
        (ROOT / target).mkdir(parents=True, exist_ok=True)
    write_seed("encode_full_width", "dense", b"\xa5")
    for label, raw in [("dense", b"\xa5"), ("carry", b"\xff"), ("sparse", b"\0"),
                       ("native", b"\1"), ("portable", b"\2"),
                       ("native-owned", b"\x82"), ("portable-owned", b"\x80")]:
        write_seed("encode_radix_full_width", label, raw)
    for length in RADIX_LENGTHS:
        for label, data in [
            ("power", b"\x01" + bytes(length - 1)),
            ("sparse", b"\x01" + bytes(length - 2) + b"\x01"),
        ]:
            write_seed("encode_all", f"radix-{label}-{length}", data)
            if length <= SMALL_MAX_BYTES:
                write_seed("encode_small", f"radix-{label}-{length}", data)
    for length in LENGTHS:
        patterned = bytes((i * 73 + 1) % 256 for i in range(length))
        for label, data in [
            ("pattern", patterned),
            ("zeros", bytes(length)),
            ("carry", b"\xff" * length),
        ]:
            write_seed("encode_all", f"{label}-{length}", data)
            if length <= SMALL_MAX_BYTES:
                write_seed("encode_small", f"{label}-{length}", data)
        for label, data in [
            ("carry", b"z" * length),
            ("zeros", b"1" * length),
            ("malformed", b"1" * (length // 2) + b"0\xff\x00" + b"z" * (length // 2)),
        ]:
            write_seed("decode_all", f"{label}-{length}", data)
            if length <= SMALL_MAX_BYTES:
                write_seed("decode_small", f"{label}-{length}", data)
    for length in DECODE_CARRY_LENGTHS:
        write_seed("decode_all", f"ntt-carry-{length}", b"z" * length)
    for length in UNICODE_LENGTHS:
        for label, char in INVALID_UNICODE:
            bad = char.encode("utf-8")
            valid_length = length - len(bad)
            for where, position in [("middle", valid_length // 2), ("end", valid_length)]:
                data = b"2" * position + bad + b"z" * (valid_length - position)
                name = f"unicode-{label}-{where}-{length}"
                write_seed("decode_all", name, data)
                if length <= SMALL_MAX_BYTES:
                    write_seed("decode_small", name, data)
    # The old first-byte selector could never choose a malformed position >255.
    late = bytearray(1024)
    late[1021] = 1
    for target in ["encode_all", "decode_all", "encode_small", "decode_small"]:
        write_seed(target, "selector-late-1024", bytes(late))
    count = (
        len(LENGTHS) * 6
        + len(DECODE_CARRY_LENGTHS)
        + sum(n <= SMALL_MAX_BYTES for n in LENGTHS) * 6
        + len(UNICODE_LENGTHS) * len(INVALID_UNICODE) * 2
        + sum(n <= SMALL_MAX_BYTES for n in UNICODE_LENGTHS) * len(INVALID_UNICODE) * 2
        + 2 * (len(RADIX_LENGTHS) + sum(n <= SMALL_MAX_BYTES for n in RADIX_LENGTHS))
        + 12
    )
    print(f"Prepared {count} deterministic seeds in {ROOT}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT,
                        help="Write additive seeds under this root; existing differing files fail.")
    ROOT = parser.parse_args().output
    main()
