#!/usr/bin/env python3
"""Synthetic wrapper checks for fail-closed coverage; these do not execute Rust."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from process_supervisor import owned_process
from run_custom_miri import skip_messages


class CoverageGuardTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix='base58-custom-miri-test-'))
        scripts = Path(__file__).resolve().parent
        repository = scripts.parent
        self.fixture = self.root / 'fixture'
        (self.fixture / 'scripts').mkdir(parents=True)
        (self.fixture / 'ci/miri').mkdir(parents=True)
        for name in ['run_custom_miri.py', 'process_supervisor.py']:
            shutil.copy2(scripts / name, self.fixture / 'scripts' / name)
        self.pins = json.loads((repository / 'ci/miri/pins.json').read_text())
        self.coverage = json.loads((repository / 'ci/miri/coverage.json').read_text())
        (self.fixture / 'ci/miri/pins.json').write_text(json.dumps(self.pins))
        self.source = self.root / 'source'
        (self.source / 'src').mkdir(parents=True)
        (self.source / 'tests').mkdir()
        (self.source / 'Cargo.toml').write_text('[package]\nname="fixture"\nversion="0.0.0"\n')
        (self.source / 'Cargo.lock').write_text('version=4\n')
        (self.source / 'src/lib.rs').write_text('// synthetic wrapper fixture\n')
        self.sysroot = self.root / 'sysroot'
        library = self.sysroot / 'lib/rustlib/x86_64-unknown-linux-gnu/lib'
        library.mkdir(parents=True)
        (library / 'libcore-fixture.rmeta').write_bytes(b'fixture; not Rust metadata')
        self.compiler = self.root / 'compiler'
        (self.compiler / 'bin').mkdir(parents=True)
        self.calls = self.root / 'wrapper-calls.jsonl'
        stub = (
            '#!/usr/bin/env python3\n'
            'import json, os, pathlib, sys\n'
            'name = pathlib.Path(sys.argv[0]).name\n'
            'if name == "cargo":\n'
            '    print("commit-hash: " + os.environ["FAKE_CARGO_COMMIT"])\n'
            'elif name in ["rustc", "miri"]:\n'
            '    print("commit-hash: " + os.environ["FAKE_RUSTC_COMMIT"])\n'
            'else:\n'
            '    with open(os.environ["FAKE_CALLS"], "a") as out:\n'
            '        out.write(json.dumps(sys.argv[1:]) + "\\n")\n'
            '    if "--list" in sys.argv:\n'
            '        for item in os.environ["FAKE_LIST"].split("|"):\n'
            '            print(item + ": test")\n'
            '    else:\n'
            '        count = 0 if os.environ.get("FAKE_ZERO") else 1\n'
            '        if os.environ.get("FAKE_SKIP") and "--show-output" in sys.argv:\n'
            '            print(os.environ["FAKE_SKIP"])\n'
            '        print(f"test result: ok. {count} passed; 0 failed; 0 ignored;")\n'
        )
        artifacts = {}
        for name in ['rustc', 'cargo', 'miri', 'cargo-miri']:
            path = self.compiler / 'bin' / name
            path.write_text(stub)
            path.chmod(0o755)
            if name in ['miri', 'cargo-miri']:
                artifacts[name] = dict(path=str(path), sha256=hashlib.sha256(path.read_bytes()).hexdigest())
        self.record = self.root / 'build.json'
        self.record.write_text(json.dumps(dict(pins=self.pins, prepared_only=False,
            compiler=str(self.compiler), cargo=str(self.compiler / 'bin/cargo'), artifacts=artifacts)))

    def run_fixture(self, suite, listed, zero=False, skip=None):
        (self.fixture / 'ci/miri/coverage.json').write_text(json.dumps(self.coverage))
        env = os.environ.copy()
        env.update(FAKE_CARGO_COMMIT=self.pins['cargo_commit'], FAKE_RUSTC_COMMIT=self.pins['compiler_commit'],
            FAKE_CALLS=str(self.calls), FAKE_LIST='|'.join(listed))
        if zero:
            env['FAKE_ZERO'] = '1'
        else:
            env.pop('FAKE_ZERO', None)
        if skip:
            env['FAKE_SKIP'] = skip
        else:
            env.pop('FAKE_SKIP', None)
        command = [sys.executable, '-B', '-O', str(self.fixture / 'scripts/run_custom_miri.py'),
            '--build-record', str(self.record), '--source', str(self.source),
            '--output', str(self.root / 'result'), '--arch', 'x86', '--suite', suite,
            '--sysroot', str(self.sysroot)]
        with owned_process(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as child:
            stdout, stderr = child.communicate(timeout=30)
        self.assertNotEqual(child.returncode, 0, (stdout, stderr, str(self.root)))
        return stderr.decode()

    def test_empty_probe_suite_cannot_pass(self):
        self.coverage['x86']['probes'] = []
        error = self.run_fixture('probes', [])
        self.assertIn('probe suite must be nonempty', error)
        self.assertFalse(self.calls.exists())

    def test_duplicate_kernel_suite_cannot_pass(self):
        name = self.coverage['x86']['library_tests'][0]
        self.coverage['x86']['library_tests'] = [name, name]
        error = self.run_fixture('kernels', [])
        self.assertIn('kernel suite must be nonempty and unique', error)
        self.assertFalse(self.calls.exists())

    def test_probe_discovery_rejects_an_omitted_test(self):
        self.coverage['x86']['probes'] = ['x86::probe']
        self.run_fixture('probes', ['x86::probe', 'x86::omitted'])
        result = json.loads((self.root / 'result/run.json').read_text())
        self.assertEqual(len(result['commands']), 1)
        self.assertEqual(result['commands'][0]['name'], 'discover-probes')
        self.assertFalse(result['commands'][0]['passed'])

    def test_zero_tests_is_failure_even_when_wrapper_exits_zero(self):
        self.coverage['x86']['probes'] = ['x86::probe']
        self.run_fixture('probes', ['x86::probe'], zero=True)
        result = json.loads((self.root / 'result/run.json').read_text())
        self.assertEqual(result['commands'][-1]['exit_code'], 0)
        self.assertFalse(result['commands'][-1]['passed'])
        self.assertNotIn('passed', result)

    def test_buffer_discovery_rejects_a_missing_shard(self):
        self.run_fixture('buffers', self.coverage['buffer_cases'][1:])
        result = json.loads((self.root / 'result/run.json').read_text())
        self.assertEqual(len(result['commands']), 1)
        self.assertEqual(result['commands'][0]['name'], 'discover-buffers')
        self.assertFalse(result['commands'][0]['passed'])

    def test_isa_skip_cannot_hide_in_one_passing_test(self):
        self.assertEqual(skip_messages('warning: skipping an unrelated Cargo build note\n'), [])
        self.check_isa_skip('skipped: no AVX-512 IFMA')

    def test_avx2_skip_cannot_hide_in_one_passing_test(self):
        self.check_isa_skip('no AVX2 on this CPU: AVX2 kernels not run')

    def test_avx2_panel_skip_cannot_hide_in_one_passing_test(self):
        self.check_isa_skip('no AVX2 on this CPU: AVX2 kernel not run')

    def test_vbmi_skip_cannot_hide_in_one_passing_test(self):
        self.check_isa_skip('no AVX-512 VBMI on this CPU: VBMI kernels not run')

    def check_isa_skip(self, message):
        self.coverage['x86']['probes'] = ['x86::probe']
        self.run_fixture('probes', ['x86::probe'], skip=message)
        result = json.loads((self.root / 'result/run.json').read_text())
        row = result['commands'][-1]
        self.assertEqual(row['exit_code'], 0)
        self.assertIn('--show-output', row['argv'])
        self.assertEqual(row['skip_messages'], [message])
        self.assertFalse(row['passed'])


if __name__ == '__main__':
    unittest.main()
