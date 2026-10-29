#!/usr/bin/env python3
"""Export independent Natural fixtures and write their exact hash manifest."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

SIZES = [0,1,8,16,21,22,23,31,32,33,63,64,65,128,256,512,1024,2048,4096,8192,
         16384,32768,65536,131072,262144,524288,1048576,2097152,4194304]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True,help='fresh fixture directory')
    parser.add_argument('--manifest',type=Path,required=True,help='fresh JSON path outside the fixture directory')
    parser.add_argument('--sizes',help='comma-separated subset; default is the full29-size grid')
    args=parser.parse_args()
    assert not args.manifest.exists()
    sizes=list(map(int,args.sizes.split(','))) if args.sizes else SIZES
    assert len(sizes)==len(set(sizes)) and all(0<=size<=4194304 for size in sizes)
    before=sha(args.binary)
    text=subprocess.check_output([str(args.binary.resolve()),str(args.output.resolve()),*map(str,sizes)],text=True)
    reported={int(size):int(length) for size,length in (line.split(',') for line in text.splitlines())}
    rows=[]
    for size in sizes:
        data,encoded=[args.output/f'{size}.{extension}' for extension in ['bin','txt']]
        assert data.stat().st_size==size and encoded.stat().st_size==reported[size]
        rows.append(dict(decoded_bytes=size,encoded_chars=reported[size],data_sha256=sha(data),encoded_sha256=sha(encoded)))
    assert sha(args.binary)==before
    record=dict(oracle='Independent Malachite Natural radix conversion; no faster-base58 conversion dependency',
        executable_sha256=before,source_sha256=sha(Path(__file__).resolve().parents[1]/'tools/fixture-export/main.rs'),fixtures=rows)
    with args.manifest.open('x') as stream:
        json.dump(record,stream,indent=2)
        stream.write('\n')
    print('Exported',len(rows),'independent fixture pairs')


if __name__=='__main__':
    main()
