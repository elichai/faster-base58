"""Pure synthetic-file tests; no compiler, shared-library loading or clocks."""
import os
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from fixed_file import Scratch, elf_load_ranges, padded_image, sha


def fake_elf(length=256, fill=0x39):
    data = bytearray([fill] * length)
    data[:16] = b'\x7fELF\x02\x01\x01' + bytes(9)
    struct.pack_into('<HHI', data, 16, 3, 62, 1)
    struct.pack_into('<Q', data, 32, 64)
    struct.pack_into('<HHH', data, 52, 64, 56, 1)
    struct.pack_into('<IIQQQQQQ', data, 64, 1, 5, 0, 0, 0, length, length, 4096)
    return bytes(data)


class FixedFileTests(unittest.TestCase):
    def setUp(self):
        # Retained deliberately: no cleanup or rm command hides a failing fixture.
        self.root = Path(tempfile.mkdtemp(prefix='base58-fixed-inode-files-'))
        self.a = self.root/'a.so'
        self.b = self.root/'b.so'
        self.a.write_bytes(fake_elf())
        self.b.write_bytes(fake_elf(384, 0x51))
        self.scratch = Scratch(self.root/'fixed.so', 512, create=True)

    def tearDown(self):
        self.scratch.close()

    def test_padding_preserves_original_and_load_ranges(self):
        image, record = padded_image(self.a, sha(self.a.read_bytes()), 512)
        self.assertEqual(image[:256], self.a.read_bytes())
        self.assertEqual(image[256:], bytes(256))
        self.assertEqual(record['load_ranges'], elf_load_ranges(self.a.read_bytes()))
        self.assertEqual(record['padding_bytes'], 256)

    def test_one_inode_and_length_across_different_images(self):
        identity = self.scratch.identity
        for path in [self.a, self.b, self.a, self.b]:
            record = self.scratch.install(path, sha(path.read_bytes()))
            self.assertEqual(record['device'], identity['device'])
            self.assertEqual(record['inode'], identity['inode'])
            self.assertEqual(self.scratch.path.stat().st_size, 512)
            self.assertEqual(sha(self.scratch.read()), record['padded_sha256'])

    def test_short_pwrite_is_completed_without_truncation(self):
        original = os.pwrite
        with patch('fixed_file.os.pwrite', side_effect=lambda fd,data,at: original(fd,data[:7],at)):
            record = self.scratch.install(self.a, sha(self.a.read_bytes()))
        self.assertEqual(sha(self.scratch.read()), record['padded_sha256'])

    def test_original_hash_failure_does_not_modify_scratch(self):
        before = self.scratch.read()
        with self.assertRaisesRegex(RuntimeError, 'Original image changed'):
            self.scratch.install(self.a, '0' * 64)
        self.assertEqual(self.scratch.read(), before)

    def test_load_window_blocks_replacement_and_close(self):
        with self.scratch.child_load_window():
            with self.assertRaisesRegex(RuntimeError, 'Cannot write'):
                self.scratch.install(self.a, sha(self.a.read_bytes()))
            with self.assertRaisesRegex(RuntimeError, 'Cannot close'):
                self.scratch.close()
        self.scratch.install(self.a, sha(self.a.read_bytes()))

    def test_reopen_requires_recorded_identity(self):
        identity = self.scratch.identity
        self.scratch.close()
        self.scratch = Scratch(self.root/'fixed.so', 512, create=False, expected_identity=identity)
        self.assertEqual(self.scratch.identity, identity)
        with self.assertRaisesRegex(RuntimeError, 'identity changed'):
            Scratch(self.root/'fixed.so', 512, create=False, expected_identity=dict(device=0,inode=0))

    def test_path_replacement_is_detected_before_write(self):
        path = self.scratch.path
        path.rename(self.root/'held.so')
        path.write_bytes(bytes([0x79])*512)
        with self.assertRaisesRegex(RuntimeError, 'path/inode changed'):
            self.scratch.install(self.a, sha(self.a.read_bytes()))
        self.assertEqual(path.read_bytes(), bytes([0x79])*512)

    def test_symlink_is_never_followed(self):
        link = self.root/'link.so'
        link.symlink_to(self.a)
        with self.assertRaises(OSError):
            Scratch(link, 512, create=True)
        with self.assertRaisesRegex(RuntimeError, 'non-symlink'):
            padded_image(link, sha(self.a.read_bytes()), 512)

    def test_no_original_may_alias_scratch(self):
        self.scratch.install(self.a, sha(self.a.read_bytes()))
        alias = self.root/'alias.so'
        os.link(self.scratch.path, alias)
        with self.assertRaisesRegex(RuntimeError, 'aliases scratch'):
            self.scratch.install(alias, sha(alias.read_bytes()))

    def test_bad_load_range_and_truncated_header_rejected(self):
        data = bytearray(fake_elf())
        struct.pack_into('<Q', data, 64+32, 257)
        with self.assertRaisesRegex(RuntimeError, 'PT_LOAD'):
            elf_load_ranges(bytes(data))
        with self.assertRaisesRegex(RuntimeError, 'image length'):
            elf_load_ranges(bytes(63))


if __name__ == '__main__':
    unittest.main()
