"""Regression tests for nightly fuzz campaign provenance."""

import json
import hashlib
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import provenance


class ProvenanceTests(unittest.TestCase):
    def test_write_records_fixed_configuration_and_refuses_replacement(self):
        with TemporaryDirectory() as directory:
            output = Path(directory) / "run.json"
            provenance.write(output, {"target": "encode_all", "cpu": "generic"})
            self.assertEqual(json.loads(output.read_text()), {"cpu": "generic", "target": "encode_all"})
            with self.assertRaises(FileExistsError):
                provenance.write(output, {"target": "decode_all"})

    def test_file_digest_is_sha256(self):
        with TemporaryDirectory() as directory:
            path = Path(directory) / "dictionary"
            path.write_bytes(b"dictionary")
            self.assertEqual(provenance.file_digest(path), hashlib.sha256(b"dictionary").hexdigest())

    def test_add_digest_uses_a_suffix_and_rejects_replacement(self):
        with TemporaryDirectory() as directory:
            path = Path(directory) / "dictionary"
            path.write_bytes(b"dictionary")
            values = {}
            provenance.add_digest(values, "dictionary", path)
            self.assertIn("dictionary_sha256", values)
            with self.assertRaises(ValueError):
                provenance.add_digest(values, "dictionary", path)


if __name__ == "__main__":
    unittest.main()
