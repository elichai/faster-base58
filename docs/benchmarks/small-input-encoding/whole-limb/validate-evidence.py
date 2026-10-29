"""Validate final source binding, measured summaries, and evidence links."""
import csv
import hashlib
import json
from pathlib import Path
import re
import statistics
import subprocess

root = Path.cwd()
evidence = root / 'docs/benchmarks/small-input-encoding/whole-limb'
arm = json.loads((evidence / 'arm-identity.json').read_text())
native = json.loads((evidence / 'x86-identity.json').read_text())
assert arm['source_hashes'] == native['source_hashes']
for name, expected in arm['source_hashes'].items():
    content = subprocess.check_output(['git', 'show', '47974045e02f7b015c4b5cccc0fca7f96f04e2c5:' + name], cwd=root)
    assert hashlib.sha256(content).hexdigest() == expected, name
assert all(row['old_candidate_final_text_equal'] for row in native['code'].values())
for row in csv.DictReader((evidence / 'summary.csv').open()):
    raw = json.loads((evidence / (row['architecture'] + '-' + row['backend'] + '.json')).read_text())
    values = [statistics.median(r['median_ns'] for r in raw if r['operation'] == row['operation'] and r['size'] == int(row['size'])
                               and r['leading_zeros'] == int(row['leading_zeros']) and r['variant'] == variant) for variant in ('baseline', 'candidate')]
    assert values == [float(row['baseline_ns']), float(row['candidate_ns'])]
    assert abs(float(row['less_time_percent']) - 100 * (1 - values[1] / values[0])) < 1e-10
for path in (evidence / 'README.md', evidence.parent / 'README.md'):
    for link in re.findall(r'\]\(([^)]+)\)', path.read_text()):
        assert (path.parent / link).exists(), (path.name, link)
print('Final source hashes, native text equivalence, summaries, and evidence links verified')
