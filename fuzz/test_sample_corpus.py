"""Checks for bounded, reproducible nightly corpus selection."""

from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import corpus_store


class SampleCorpusTests(unittest.TestCase):
    def test_sample_is_bounded_reproducible_and_seeded(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            for target in corpus_store.TARGETS:
                (source / target).mkdir(parents=True)
                for i in range(100):
                    data = str(i).encode()
                    (source / target / corpus_store.digest(data)).write_bytes(data)
            before = {path.relative_to(source): path.read_bytes()
                      for path in source.glob("*/*")}
            for name, seed in [("first", "123"), ("repeat", "123"), ("next", "124")]:
                counts = corpus_store.sample(source, root / name, limit=8, seed=seed)
                self.assertEqual(counts, dict.fromkeys(corpus_store.TARGETS, 8))
                corpus_store.validate(root / name)
            for target in corpus_store.TARGETS:
                first = {p.name for p in (root / "first" / target).iterdir()}
                repeat = {p.name for p in (root / "repeat" / target).iterdir()}
                following = {p.name for p in (root / "next" / target).iterdir()}
                self.assertEqual(first, repeat)
                self.assertNotEqual(first, following)
                self.assertTrue(first <= {p.name for p in (source / target).iterdir()})
            self.assertEqual(before, {path.relative_to(source): path.read_bytes()
                                      for path in source.glob("*/*")})

    def test_empty_and_small_corpora_and_existing_output(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            data = b"input"
            (source / "encode_all").mkdir()
            (source / "encode_all" / corpus_store.digest(data)).write_bytes(data)
            counts = corpus_store.sample(source, root / "sample", limit=8, seed="1")
            self.assertEqual(counts, dict.fromkeys(corpus_store.TARGETS, 0) | {"encode_all": 1})
            with self.assertRaises(FileExistsError):
                corpus_store.sample(source, root / "sample", limit=8, seed="2")
            for limit in (0, -1):
                with self.assertRaises(ValueError):
                    corpus_store.sample(source, root / "invalid", limit=limit, seed="1")
            self.assertFalse((root / "invalid").exists())

    def test_validates_all_inputs_before_sampling(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source" / "encode_all"
            source.mkdir(parents=True)
            (source / "bad-name").write_bytes(b"input")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                corpus_store.sample(source.parent, root / "sample", limit=1, seed="1")
            self.assertFalse((root / "sample").exists())


if __name__ == "__main__":
    unittest.main()
