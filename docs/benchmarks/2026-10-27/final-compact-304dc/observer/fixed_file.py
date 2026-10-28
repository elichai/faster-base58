"""Single-inode ELF replacement between fully reaped measurement children."""
from contextlib import contextmanager
import hashlib
import os
from pathlib import Path
import struct

MAX_IMAGE_BYTES = 16 * 1024 * 1024


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def elf_load_ranges(data):
    """Validate the ELF64/x86-64 shared-library ranges before adding trailing zeros."""
    require(64 <= len(data) <= MAX_IMAGE_BYTES, 'Unsupported image length')
    require(data[:6] == b'\x7fELF\x02\x01', 'Require little-endian ELF64')
    require(struct.unpack_from('<HH', data, 16) == (3, 62), 'Require x86-64 ET_DYN')
    offset = struct.unpack_from('<Q', data, 32)[0]
    stride, count = struct.unpack_from('<HH', data, 54)
    require(stride == 56 and 0 < count < 1024, 'Unsupported program-header table')
    require(64 <= offset and offset + stride * count <= len(data), 'Program headers exceed original image')
    ranges = []
    for index in range(count):
        kind, flags, start, virtual, physical, size, memory, alignment = struct.unpack_from('<IIQQQQQQ', data, offset + stride * index)
        if kind == 1:
            require(size <= memory and start + size <= len(data), 'PT_LOAD exceeds original image')
            ranges.append(dict(file_offset=start, virtual_address=virtual, file_bytes=size, memory_bytes=memory, flags=flags))
    require(bool(ranges), 'No PT_LOAD segments')
    return ranges


def padded_image(path, expected_sha256, global_length):
    path = Path(path)
    require(path.is_file() and not path.is_symlink(), 'Original image must be a regular non-symlink file')
    original = path.read_bytes()
    require(sha(original) == expected_sha256, 'Original image changed')
    ranges = elf_load_ranges(original)
    require(len(original) <= global_length <= MAX_IMAGE_BYTES, 'Bad global image length')
    padded = original + bytes(global_length - len(original))
    return padded, dict(original_sha256=expected_sha256, original_bytes=len(original),
                        padded_sha256=sha(padded), padded_bytes=global_length,
                        padding_bytes=global_length-len(original), load_ranges=ranges)


class Scratch:
    """One fixed path and descriptor; a load-window lease forbids replacement."""
    def __init__(self, path, length, *, create, expected_identity=None):
        self.path = Path(path)
        self.length = length
        self.busy = False
        self.closed = False
        require(64 <= length <= MAX_IMAGE_BYTES, 'Bad scratch size')
        flags = os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW
        if create:
            flags |= os.O_CREAT | os.O_EXCL
        self.fd = os.open(self.path, flags, 0o600)
        try:
            if create:
                remaining = memoryview(bytes(length))
                while remaining:
                    written = os.write(self.fd, remaining)
                    require(written > 0, 'Incomplete initial scratch allocation')
                    remaining = remaining[written:]
                os.fsync(self.fd)
            self.identity = self._identity()
            if expected_identity is not None:
                require(self.identity == expected_identity, 'Scratch identity changed between phases')
            self.verify_identity()
        except BaseException:
            os.close(self.fd)
            self.closed = True
            raise

    def _identity(self):
        stat = os.fstat(self.fd)
        return dict(device=stat.st_dev, inode=stat.st_ino)

    def verify_identity(self):
        require(not self.closed, 'Scratch is closed')
        stat = os.stat(self.path, follow_symlinks=False)
        require(not self.path.is_symlink(), 'Scratch path became a symlink')
        require(dict(device=stat.st_dev, inode=stat.st_ino) == self.identity == self._identity(), 'Scratch path/inode changed')
        require(stat.st_size == os.fstat(self.fd).st_size == self.length, 'Scratch length changed')
        return self.identity

    def read(self):
        self.verify_identity()
        result = bytearray()
        while len(result) < self.length:
            chunk = os.pread(self.fd, self.length-len(result), len(result))
            require(bool(chunk), 'Short scratch read')
            result.extend(chunk)
        self.verify_identity()
        return bytes(result)

    def install(self, original, expected_sha256):
        require(not self.busy, 'Cannot write while a child load-window is active')
        self.verify_identity()
        image, metadata = padded_image(original, expected_sha256, self.length)
        source = os.stat(original, follow_symlinks=False)
        require(dict(device=source.st_dev, inode=source.st_ino) != self.identity, 'Original aliases scratch')
        at = 0
        while at < len(image):
            written = os.pwrite(self.fd, memoryview(image)[at:], at)
            require(written > 0, 'Short scratch write')
            at += written
        os.fsync(self.fd)
        self.verify_identity()
        require(self.read() == image, 'Installed image verification failed')
        require(sha(Path(original).read_bytes()) == expected_sha256, 'Original changed during install')
        return dict(**metadata, **self.identity)

    @contextmanager
    def child_load_window(self):
        require(not self.busy, 'Nested child load-window')
        self.verify_identity()
        self.busy = True
        try:
            yield
        finally:
            # The controller must nest its owned_process context inside this lease.
            # Its cleanup/reap therefore completes before replacement is possible.
            self.busy = False
            self.verify_identity()

    def close(self):
        require(not self.busy, 'Cannot close during a child load-window')
        if not self.closed:
            os.close(self.fd)
            self.closed = True
