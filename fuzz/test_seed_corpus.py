"""Regression checks for boundary seeds and preservation of manual corpus edits."""

from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

import seed_corpus


class SeedCorpusTests(unittest.TestCase):
    def test_radix_seeds_cover_zero_subtrees_and_sparse_carries(self):
        with TemporaryDirectory() as directory:
            previous = seed_corpus.ROOT
            seed_corpus.ROOT = root = Path(directory)
            try:
                with redirect_stdout(StringIO()):
                    seed_corpus.main()
            finally:
                seed_corpus.ROOT = previous
            for length in [33, 65, 129, 65_537]:
                power = (root / "encode_all" / f"radix-power-{length}").read_bytes()
                sparse = (root / "encode_all" / f"radix-sparse-{length}").read_bytes()
                self.assertEqual(power, b"\x01" + bytes(length - 1))
                self.assertEqual(sparse, b"\x01" + bytes(length - 2) + b"\x01")
                if length <= 2048:
                    self.assertEqual((root / "encode_small" / f"radix-power-{length}").read_bytes(), power)
                    self.assertEqual((root / "encode_small" / f"radix-sparse-{length}").read_bytes(), sparse)

    def test_unicode_seeds_cover_late_byte_indices_and_preserve_budget(self):
        with TemporaryDirectory() as directory:
            previous = seed_corpus.ROOT
            seed_corpus.ROOT = root = Path(directory)
            try:
                with redirect_stdout(StringIO()):
                    seed_corpus.main()
            finally:
                seed_corpus.ROOT = previous
            for length in seed_corpus.UNICODE_LENGTHS:
                for label, char in seed_corpus.INVALID_UNICODE:
                    for where in ["middle", "end"]:
                        data = (root / "decode_all" / f"unicode-{label}-{where}-{length}").read_bytes()
                        self.assertEqual(len(data), length)
                        self.assertLessEqual(len(data), seed_corpus.DECODE_MAX_BYTES)
                        self.assertEqual(data.decode("utf-8").count(char), 1)
                        index = data.index(char.encode("utf-8"))
                        valid = length - len(char.encode("utf-8"))
                        self.assertEqual(index, valid // 2 if where == "middle" else valid)
                        if length == 8193:
                            self.assertGreater(index, 255)
            late = (root / "decode_all" / "selector-late-1024").read_bytes()
            self.assertEqual(late[:4], b"\0" * 4)
            self.assertEqual(late[1021:], b"\1\0\0")

    def test_carry_seeds_cross_limb_boundary_and_cover_encode_budget(self):
        # n copies of z represent exactly 58**n - 1. These straddle 8192 u64 limbs.
        self.assertLessEqual((58**89_499 - 1).bit_length(), 8192 * 64)
        self.assertGreater((58**89_500 - 1).bit_length(), 8192 * 64)
        # 65,537 all-ff bytes require 89,502 digits, not 89,501.
        value = (1 << (65_537 * 8)) - 1
        self.assertGreaterEqual(value, 58**89_501)
        self.assertLess(value, 58**89_502)
        with TemporaryDirectory() as directory:
            previous = seed_corpus.ROOT
            seed_corpus.ROOT = root = Path(directory)
            try:
                with redirect_stdout(StringIO()):
                    seed_corpus.main()
            finally:
                seed_corpus.ROOT = previous
            # Keep the old named seeds as well as both sides of the new boundary.
            for length in (44_760, 44_761, 89_492, 89_493, 89_499, 89_500, 89_501, 89_502):
                with self.subTest(length=length):
                    self.assertEqual(
                        (root / "decode_all" / f"ntt-carry-{length}").read_bytes(),
                        b"z" * length,
                    )
            self.assertEqual(
                (root / "encode_all" / "carry-65537").read_bytes(), b"\xff" * 65_537,
            )
            self.assertEqual((root / "encode_full_width" / "dense").read_bytes(), b"\xa5")
            for name, raw in [("dense", b"\xa5"), ("carry", b"\xff"), ("sparse", b"\0"),
                              ("native", b"\1"), ("portable", b"\2"),
                              ("native-owned", b"\x82"), ("portable-owned", b"\x80")]:
                self.assertEqual((root / "encode_radix_full_width" / name).read_bytes(), raw)

    def test_existing_seed_edits_are_rejected(self):
        with TemporaryDirectory() as directory:
            previous = seed_corpus.ROOT
            seed_corpus.ROOT = root = Path(directory)
            (root / "encode_all").mkdir()
            seed = root / "encode_all" / "edited-seed"
            seed.write_bytes(b"manual edit")
            try:
                with self.assertRaisesRegex(ValueError, "manual changes"):
                    seed_corpus.write_seed("encode_all", "edited-seed", b"expected")
                self.assertEqual(seed.read_bytes(), b"manual edit")
            finally:
                seed_corpus.ROOT = previous

    def test_existing_seed_edits_are_rejected_with_python_optimization(self):
        result = subprocess.run(
            [sys.executable, "-O", str(Path(__file__).resolve()),
             "SeedCorpusTests.test_existing_seed_edits_are_rejected"],
            check=False, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
