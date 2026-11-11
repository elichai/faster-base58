"""Regression tests for the oracle RUSTC_WRAPPER."""

import unittest

from oracle_rustc import filter_args

FUZZ_FLAGS = [
    "-Cpasses=sancov-module",
    "-Cllvm-args=-sanitizer-coverage-level=4",
    "-Cllvm-args=-sanitizer-coverage-inline-8bit-counters",
    "-Cinstrument-coverage",
    "-Zsanitizer=address",
    "-Cdebug-assertions",
    "-C",
    "overflow-checks=on",
    "-C",
    "opt-level=3",
    "--cfg",
    "fuzzing",
]


class OracleRustcTests(unittest.TestCase):
    def test_oracle_crates_lose_only_fuzz_instrumentation(self):
        for crate in ("malachite_base", "malachite_nz", "bs58"):
            args = ["--crate-name", crate, "src/lib.rs", *FUZZ_FLAGS]
            self.assertEqual(
                filter_args(args),
                ["--crate-name", crate, "src/lib.rs", "-Zsanitizer=address", "-C", "opt-level=3", "--cfg", "fuzzing"],
            )

    def test_other_crates_are_unchanged(self):
        for crate in ("faster_base58", "encode_all", "malachite"):
            args = ["--crate-name", crate, *FUZZ_FLAGS]
            self.assertEqual(filter_args(args), args)

    def test_calls_without_a_crate_name_are_unchanged(self):
        for args in (["-vV"], ["--print", "sysroot"], ["-", "--crate-name"]):
            self.assertEqual(filter_args(args), args)


if __name__ == "__main__":
    unittest.main()
