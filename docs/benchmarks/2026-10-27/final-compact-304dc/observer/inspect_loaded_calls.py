#!/usr/bin/env python3
"""Untimed exact-target/argument and mapped-code inspection of calibration v1."""
import argparse
import ctypes
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import subprocess
import sys

ORIGINAL_CDLL = ctypes.CDLL
EVENTS = []


def digest(data):
    return hashlib.sha256(data).hexdigest()


class Elf:
    def __init__(self, path):
        self.path = path
        self.data = path.read_bytes()
        assert self.data[:6] == b'\x7fELF\x02\x01' and struct.unpack_from('<H', self.data, 18)[0] == 62
        offset = struct.unpack_from('<Q', self.data, 40)[0]
        stride, count, strings = struct.unpack_from('<HHH', self.data, 58)
        self.sections = [struct.unpack_from('<IIQQQQIIQQ', self.data, offset + n * stride) for n in range(count)]
        names = self.section_data(self.sections[strings])
        self.by_name = {names[s[0]:].split(b'\0', 1)[0].decode(): s for s in self.sections}
        self.symbols = {}
        for section in self.sections:
            if section[1] not in [2, 11]:
                continue
            table = self.read_symbols(section)
            for symbol in table:
                if symbol['name'] and symbol['size']:
                    self.symbols[symbol['name']] = symbol
        self.dynsym = self.read_symbols(self.by_name['.dynsym'])

    def section_data(self, section):
        return self.data[section[4]:section[4] + section[5]]

    def read_symbols(self, section):
        names = self.section_data(self.sections[section[6]])
        symbols = []
        for at in range(section[4], section[4] + section[5], section[9]):
            name, info, other, index, address, size = struct.unpack_from('<IBBHQQ', self.data, at)
            symbols.append(dict(name=names[name:].split(b'\0', 1)[0].decode(), info=info, other=other,
                                index=index, address=address, size=size))
        return symbols


class DlInfo(ctypes.Structure):
    _fields_ = [('fname', ctypes.c_char_p), ('fbase', ctypes.c_void_p),
                ('sname', ctypes.c_char_p), ('saddr', ctypes.c_void_p)]


def resolve(address):
    if not address:
        return dict(address=0)
    library = ORIGINAL_CDLL(None)
    query = library.dladdr
    query.argtypes, query.restype = (ctypes.c_void_p, ctypes.POINTER(DlInfo)), ctypes.c_int
    value = DlInfo()
    if not query(address, ctypes.byref(value)):
        return dict(address=address, resolved=False)
    return dict(address=address, library=value.fname.decode() if value.fname else None,
                module_base=value.fbase, module_offset=address - value.fbase,
                symbol=value.sname.decode() if value.sname else None, symbol_address=value.saddr)


class Loaded:
    def __init__(self, name, *args, **kwargs):
        self.real = ORIGINAL_CDLL(name, *args, **kwargs)
        self.path = Path(name)
        self.elf = Elf(self.path)
        self.functions = {}
        pointer = ctypes.cast(getattr(self.real, 'size_encode'), ctypes.c_void_p).value
        self.base = pointer - self.elf.symbols['size_encode']['address']
        assert self.base % 4096 == 0

    def __getattr__(self, name):
        if name.startswith('size_'):
            if name not in self.functions:
                self.functions[name] = Traced(getattr(self.real, name), name, self)
            return self.functions[name]
        return getattr(self.real, name)

    def state(self):
        result = []
        for name, symbol in self.elf.symbols.items():
            if not ('faster_base58' in name and any(token in name for token in ['STORAGE', 'CACHE', 'TABLE'])):
                continue
            index = symbol['index']
            if not (0 < index < len(self.elf.sections)):
                continue
            section = self.elf.sections[index]
            if symbol['info'] & 15 != 1 or section[2] & 3 != 3:
                continue
            data = ctypes.string_at(self.base + symbol['address'], symbol['size'])
            result.append(dict(name=name, offset=symbol['address'], bytes=symbol['size'],
                               sha256=digest(data), prefix_hex=data[:64].hex()))
        return result

    def mapped(self):
        result = {}
        for name in ['.text', '.rodata']:
            section = self.elf.by_name[name]
            actual = ctypes.string_at(self.base + section[3], section[5])
            original = self.elf.section_data(section)
            result[name] = dict(bytes=len(actual), file_sha256=digest(original),
                                memory_sha256=digest(actual), identical=actual == original)
        return result

    def relocations(self):
        result = []
        for section in self.elf.sections:
            if section[1] != 4 or section[6] != self.elf.sections.index(self.elf.by_name['.dynsym']):
                continue
            for at in range(section[4], section[4] + section[5], section[9]):
                address, info, addend = struct.unpack_from('<QQq', self.elf.data, at)
                kind, index = info & 0xffffffff, info >> 32
                if kind not in [6, 7]:
                    continue
                symbol = self.elf.dynsym[index]['name']
                actual = ctypes.c_uint64.from_address(self.base + address).value
                result.append(dict(offset=address, kind=kind, symbol=symbol, actual=resolve(actual)))
        return result


class Traced:
    def __init__(self, real, name, owner):
        object.__setattr__(self, 'real', real)
        object.__setattr__(self, 'name', name)
        object.__setattr__(self, 'owner', owner)

    def __getattr__(self, name):
        return getattr(self.real, name)

    def __setattr__(self, name, value):
        setattr(self.real, name, value)

    @property
    def _as_parameter_(self):
        return self.real

    def __call__(self, *args):
        pointer = ctypes.cast(self.real, ctypes.c_void_p).value
        symbol = self.owner.elf.symbols[self.name]
        assert pointer == self.owner.base + symbol['address']
        section = next(s for s in self.owner.elf.sections if s[1] != 8 and s[3] <= symbol['address'] and symbol['address'] + symbol['size'] <= s[3] + s[5])
        file_offset = section[4] + symbol['address'] - section[3]
        actual = ctypes.string_at(pointer, symbol['size'])
        original = self.owner.elf.data[file_offset:file_offset + symbol['size']]
        input_data = ctypes.string_at(args[0], args[1])
        event = dict(export=self.name, pointer=pointer, module_offset=pointer - self.owner.base,
                     signature=[dict(type=t.__name__, bytes=ctypes.sizeof(t)) for t in self.real.argtypes],
                     return_type=self.real.restype.__name__, arguments=list(args),
                     input_bytes=len(input_data), input_sha256=digest(input_data), input_prefix_hex=input_data[:256].hex(),
                     output_capacity_argument=args[3], repeat_iteration_argument=args[4] if len(args) == 5 else None,
                     function_bytes=len(actual), function_file_sha256=digest(original), function_memory_sha256=digest(actual),
                     function_identical=actual == original, mutable_before=self.owner.state(),
                     mapped_before=self.owner.mapped(), targets_before=self.owner.relocations())
        EVENTS.append(event)
        result = self.real(*args)
        event.update(result=result, mutable_after=self.owner.state(), mapped_after=self.owner.mapped(), targets_after=self.owner.relocations())
        return result


def traced_cdll(name, *args, **kwargs):
    return Loaded(name, *args, **kwargs) if name is not None and str(name).endswith('-both-cdylib.so') else ORIGINAL_CDLL(name, *args, **kwargs)


def main():
    if sys.flags.optimize:
        raise SystemExit('Require Python optimize0')
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--root', type=Path, required=True)
    p.add_argument('--label', choices=['base', 'dc3072', 'pair-a', 'pair-b'], required=True)
    p.add_argument('--operation', choices=['encode', 'decode'], required=True)
    p.add_argument('--size', type=int, required=True)
    args = p.parse_args()
    original = args.root / 'drivers/calibrate_identical.py'
    assert digest(original.read_bytes()) == '8d7d536d1ff0924b9f95719bd451c8c3d278bc17d43143722eea2a09d99e31ae'
    spec = importlib.util.spec_from_file_location('sealed_calibration_v1', original)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    args.worker, args.backend, args.affinity, args.buffers, args.floor_ns = 'qualify', 'auto', 'pinned', 'fixed', 100_000_000
    args.labels = 'original' if args.label in ['base', 'dc3072'] else 'equal'
    ctypes.CDLL = traced_cdll
    value = module.worker(args)
    assert len(EVENTS) == 3
    assert [row['export'] for row in EVENTS] == ['size_' + args.operation] + ['size_repeat_encode' if args.operation == 'encode' else 'size_repeat_decode_prevalidated'] * 2
    assert [row['repeat_iteration_argument'] for row in EVENTS] == [None, 0, 1]
    assert all(row['input_sha256'] == value['input_sha256'] and row['output_capacity_argument'] == value['output_capacity'] for row in EVENTS)
    print(json.dumps(dict(status='untimed diagnostic', script_sha256=digest(Path(__file__).read_bytes()),
                          original_observer_sha256=digest(original.read_bytes()), worker=value, calls=EVENTS,
                          limitation='Separate instrumented qualification, not a retime or hardware register trace. Logs actual ctypes callable pointer/primitive args and mapped bytes/targets; no host setting changed.')))


if __name__ == '__main__':
    main()
