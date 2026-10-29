"""Bind the final chunk-iterator spelling to the measured canonical machine code."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess

old = Path('/private/tmp/base58-pr1-canonical-arm')
output = Path('/private/tmp/base58-pr1-final-arm')
output.mkdir(exist_ok=False)
probe = Path('/private/tmp/base58-pr1-size-pr')
source = Path('/private/tmp/base58-pr1-review')
metadata = json.loads((probe / 'metadata.json').read_text())
env = dict(os.environ, CARGO_TARGET_DIR=str(probe / 'target'), RUSTFLAGS=metadata['rustflags'])
env.pop('CARGO_ENCODED_RUSTFLAGS', None)
def digest(data):
    return hashlib.sha256(data).hexdigest()
def code_sections(data):
    assert data[:4] == b'\xcf\xfa\xed\xfe'
    offset = 32
    sections = {}
    for _ in range(struct.unpack_from('<I', data, 16)[0]):
        command, length = struct.unpack_from('<II', data, offset)
        if command == 0x19:
            for index in range(struct.unpack_from('<I', data, offset + 64)[0]):
                section = offset + 72 + 80 * index
                flags = struct.unpack_from('<I', data, section + 64)[0]
                if flags & 0x80000400 or flags & 255 == 8:
                    size = struct.unpack_from('<Q', data, section + 40)[0]
                    start = struct.unpack_from('<I', data, section + 48)[0]
                    sections[data[section:section + 16].split(b'\0')[0].decode()] = data[start:start + size]
        offset += length
    assert '__text' in sections
    return sections
proof = {'source_hashes': {str(path.relative_to(source)): digest(path.read_bytes()) for path in (source / 'src/encode/mod.rs', source / 'src/encode/matrix.rs', source / 'src/encode/tests.rs')},
         'artifacts': {}}
for backend, command in zip(('auto', 'no-alloc'), metadata['commands']):
    subprocess.run(command, env=env, check=True)
    path = output / (backend + '-both-cdylib.dylib')
    shutil.copyfile(probe / 'target/aarch64-apple-darwin/release/libbase58_code_size.dylib', path)
    before = (old / path.name).read_bytes()
    after = path.read_bytes()
    before_sections = code_sections(before)
    after_sections = code_sections(after)
    same = before_sections == after_sections
    proof['artifacts'][backend] = {'old_sha256': digest(before), 'final_sha256': digest(after), 'all_code_sections_equal': same,
        'old_sections': {name: digest(data) for name, data in before_sections.items()},
        'final_sections': {name: digest(data) for name, data in after_sections.items()}}
    print(backend, 'byte-identical executable sections:', same, flush=True)
(output / 'equivalence.json').write_text(json.dumps(proof, indent=2) + '\n')
