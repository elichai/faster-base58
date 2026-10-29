"""Reuse the existing fresh-process control protocol for canonical final fixed inputs."""
from pathlib import Path

script = Path('/dev/shm/base58_pr1_controls.py').read_text()
replacements = {
    "'/dev/shm/base58-pr1-improve'": "'/dev/shm/base58-pr1-canonical'",
    "('generic', 'native')": "('native',)",
    "('base58-pr1-reference-' + cpu)": "'reference'",
    "('base58-pr1-ten-top-' + cpu)": "'final'",
    "'top-fresh-controls.json'": "'final-fixed-controls.json'",
    "'encode-random-16'": "'encode-random-64'",
    "    data = bytes(size) if pattern == 'zero' else random.Random(size).randbytes(size)":
        "    rng = random.Random(size)\n    data = bytes(size) if pattern == 'zero' else bytes(rng.randrange(1, 256) for _ in range(size))",
}
for before, after in replacements.items():
    assert before in script, before
    script = script.replace(before, after)
exec(compile(script, __file__, 'exec'))
