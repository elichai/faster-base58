"""Tests for the untrusted-artifact corpus staging boundary."""

from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import corpus_store


class CorpusStoreTests(unittest.TestCase):
    def test_stage_canonicalizes_duplicate_bytes_by_sha256(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            incoming = root / "incoming" / "encode_all"
            incoming.mkdir(parents=True)
            (incoming / "first").write_bytes(b"same input")
            (incoming / "second").write_bytes(b"same input")

            added = corpus_store.stage(root / "incoming", root / "corpora")

            digest = corpus_store.digest(b"same input")
            self.assertEqual(added, {"encode_all": [digest]})
            self.assertEqual((root / "corpora" / "encode_all" / digest).read_bytes(), b"same input")

    def test_radix_target_can_be_staged_and_validated(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            incoming = root / "incoming" / "encode_radix_full_width"
            incoming.mkdir(parents=True)
            (incoming / "dense").write_bytes(b"\xa5")

            added = corpus_store.stage(root / "incoming", root / "corpora")

            digest = corpus_store.digest(b"\xa5")
            self.assertEqual(added, {"encode_radix_full_width": [digest]})
            self.assertEqual((root / "corpora" / "encode_radix_full_width" / digest).read_bytes(), b"\xa5")
            corpus_store.validate(root / "corpora")

    def test_stage_rejects_symlinks_and_oversized_files(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            incoming = root / "incoming" / "decode_all"
            incoming.mkdir(parents=True)
            (incoming / "too-large").write_bytes(b"abc")
            (incoming / "link").symlink_to(incoming / "too-large")

            with self.assertRaisesRegex(ValueError, "regular file"):
                corpus_store.stage(root / "incoming", root / "corpora", max_bytes=2)

    def test_stage_rejects_a_symlinked_destination_root(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            incoming = root / "incoming" / "encode_all"
            incoming.mkdir(parents=True)
            (incoming / "input").write_bytes(b"input")
            target = root / "outside"
            target.mkdir()
            (root / "corpora").symlink_to(target, target_is_directory=True)

            with self.assertRaisesRegex(ValueError, "corpus root"):
                corpus_store.stage(root / "incoming", root / "corpora")
            self.assertFalse((target / "encode_all").exists())

    def test_validate_requires_canonical_names_for_replay(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            corpus = root / "corpora" / "encode_full_width"
            corpus.mkdir(parents=True)
            (corpus / "not-a-digest").write_bytes(b"data")

            with self.assertRaisesRegex(ValueError, "SHA-256"):
                corpus_store.validate(root / "corpora")

    def test_validate_enforces_a_total_item_budget(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            corpus = root / "corpora" / "encode_all"
            corpus.mkdir(parents=True)
            for data in [b"first", b"second"]:
                (corpus / corpus_store.digest(data)).write_bytes(data)

            with self.assertRaisesRegex(ValueError, "item limit"):
                corpus_store.validate(root / "corpora", max_items=1)

    def test_stage_rejects_a_noncanonical_target_directory(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            extra = root / "incoming" / "encode_small"
            extra.mkdir(parents=True)
            (extra / "input").write_bytes(b"input")

            with self.assertRaisesRegex(ValueError, "unsupported corpus target"):
                corpus_store.stage(root / "incoming", root / "corpora")


if __name__ == "__main__":
    unittest.main()
