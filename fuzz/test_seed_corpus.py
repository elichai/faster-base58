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
