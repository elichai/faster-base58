"""Standard-library checks for the retained documentation and external inputs."""
import csv
import tempfile
import subprocess
import sys
import unittest
from pathlib import Path

from current import SIZES, check, tables, timing_cells
import render_backends


class DocumentationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / 'assets/graphs').mkdir(parents=True)
        (self.root / 'docs').mkdir()
        for name in ['README.md', 'DESIGN.md', 'CORRECTNESS.md', 'REFERENCES.md',
                     'docs/README.md', 'docs/performance.md']:
            (self.root / name).write_text('')
        for name in ['m5-public', 'wasm-public', 'gcp-generic-backends', 'gcp-native-backends']:
            (self.root / f'assets/graphs/{name}.svg').write_text(
                '<svg xmlns="http://www.w3.org/2000/svg"><title>Example</title></svg>')
        (self.root / 'README.md').write_text('\n'.join(
            f'![figure](assets/graphs/{name}.svg)' for name in
            ['m5-public', 'wasm-public', 'gcp-generic-backends']))
        (self.root / 'docs/performance.md').write_text(
            '![native](../assets/graphs/gcp-native-backends.svg)')

    def test_checks_retained_documents_without_raw_measurements(self):
        self.assertEqual(check(self.root)['svg_figures'], 4)

    def test_historical_native_figure_is_optional(self):
        (self.root / 'docs/performance.md').write_text('Historical comparison is tabulated only.')
        (self.root / 'assets/graphs/gcp-native-backends.svg').rename(self.root / 'historical.svg')
        self.assertEqual(check(self.root)['svg_figures'], 3)

    def test_missing_local_link_is_rejected(self):
        (self.root / 'DESIGN.md').write_text('[missing](docs/missing.md)')
        with self.assertRaisesRegex(ValueError, 'missing.md'):
            check(self.root)

    def test_invalid_svg_is_rejected(self):
        (self.root / 'assets/graphs/m5-public.svg').write_text('<svg>')
        with self.assertRaisesRegex(ValueError, 'SVG'):
            check(self.root)

    def test_readme_must_embed_selected_figure(self):
        guide = self.root / 'README.md'
        guide.write_text(guide.read_text().replace('![figure](assets/graphs/m5-public.svg)', ''))
        with self.assertRaisesRegex(ValueError, 'README.*m5-public'):
            check(self.root)

    def test_native_figure_stays_in_performance_document(self):
        guide = self.root / 'README.md'
        guide.write_text(guide.read_text() + '\n![native](assets/graphs/gcp-native-backends.svg)')
        with self.assertRaisesRegex(ValueError, 'native.*performance'):
            check(self.root)

    def test_public_x86_figure_can_replace_historical_backend_figure(self):
        guide = self.root / 'README.md'
        guide.write_text(guide.read_text().replace('gcp-generic-backends.svg', 'gcp-public.svg'))
        (self.root / 'assets/graphs/gcp-public.svg').write_text(
            '<svg xmlns="http://www.w3.org/2000/svg"><title>Public x86</title></svg>')
        self.assertEqual(check(self.root)['status'], 'pass')

    def test_reference_and_encoded_links_are_checked(self):
        (self.root / 'DESIGN.md').write_text('[reference][target]\n\n[target]: docs/missing%20file.md')
        with self.assertRaisesRegex(ValueError, 'missing file.md'):
            check(self.root)


class ExternalInputTests(unittest.TestCase):
    def rows(self, platforms=('m5', 'wasm')):
        return [dict(platform=platform, cache=cache, backend=backend, operation=operation,
                     decoded_bytes=str(size), status='measured', reason='',
                     role={'auto': 'public', 'no-alloc': 'public', 'bs58': 'competitor',
                           'five8': 'competitor', 'empty': 'scaffold'}[backend], source_fingerprint='a' * 64,
                     scope='ordinary public consumer; no __internals' if platform != 'wasm'
                     else 'ordinary public Wasm module; no __internals',
                     primary_clock='process CPU' if platform != 'wasm' else 'elapsed',
                     warm_ns='100', process_min_ns='90', process_max_ns='110',
                     fresh_processes='3', warm_batches_per_process='3')
                for platform in platforms for cache in ['default', 'cache']
                for backend in ['empty', 'auto', 'no-alloc', 'bs58', 'five8']
                for operation in ['encode', 'decode'] for size in SIZES]

    def test_missing_measurements_are_not_silent_skips(self):
        with self.assertRaisesRegex(ValueError, 'Missing.*timing'):
            tables([], [])

    def test_generic_x86_public_measurements_use_native_clock_and_size_sections(self):
        rows = self.rows(('m5', 'gcp', 'wasm'))
        cells = timing_cells(rows)
        self.assertEqual(cells['gcp', 'default', 'auto', 'encode', 32]['primary_clock'], 'process CPU')
        sizes = [dict(platform=platform, cache=cache, backend=backend, direction='both',
                      delta_empty_code_bytes='1024', delta_empty_readonly_bytes='2048',
                      delta_empty_wasm_code_section_bytes='3072', delta_empty_wasm_data_section_bytes='4096')
                 for platform in ['m5', 'gcp', 'wasm']
                 for backend, cache in [('auto', 'default'), ('auto', 'cache'), ('no-alloc', 'default'), ('bs58', 'default')]]
        output = tables(rows, sizes)
        self.assertIn('### AMD EPYC 9B45 (generic x86-64)', output)
        self.assertIn('| Auto | 1.0 KiB / 2.0 KiB | 1.0 KiB / 2.0 KiB | 3.0 KiB / 4.0 KiB |', output)
        next(row for row in rows if row['platform'] == 'gcp')['primary_clock'] = 'elapsed'
        with self.assertRaises(ValueError):
            timing_cells(rows)

    def test_renderer_requires_explicit_external_inputs(self):
        result = subprocess.run([sys.executable, str(Path(__file__).with_name('render_current.py')), '--help'],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('--data', result.stdout)
        self.assertIn('--sizes', result.stdout)

    def test_duplicate_measurements_are_rejected(self):
        rows = self.rows()
        with self.assertRaisesRegex(ValueError, 'Duplicate timing'):
            timing_cells(rows + [rows[0]])

    def test_invalid_range_and_unexplained_skip_are_rejected(self):
        for changes in [dict(warm_ns='nan'), dict(process_max_ns='80'),
                        dict(status='skipped', reason=''), dict(status='skipped', reason='policy cap')]:
            with self.subTest(changes=changes):
                rows = self.rows()
                rows[0].update(changes)
                with self.assertRaises(ValueError):
                    timing_cells(rows)

    def test_mixed_source_or_internal_scope_is_rejected(self):
        for changes in [dict(scope='internal backend'), dict(role='control'),
                        dict(source_fingerprint='b' * 64), dict(source_commit='b' * 40),
                        dict(measurement_date='2026-10-29')]:
            with self.subTest(changes=changes):
                rows = self.rows()
                rows[0].update(changes)
                with self.assertRaises(ValueError):
                    timing_cells(rows)

    def test_explicit_skip_produces_dash_without_bolding(self):
        rows = self.rows()
        for row in rows:
            if row['backend'] == 'five8':
                row.update(status='skipped', reason='unsupported size', warm_ns='',
                           process_min_ns='', process_max_ns='', primary_clock='')
        sizes = [dict(platform=platform, cache=cache, backend=backend, direction='both',
                      delta_empty_code_bytes='1024', delta_empty_readonly_bytes='2048',
                      delta_empty_wasm_code_section_bytes='3072', delta_empty_wasm_data_section_bytes='4096')
                 for platform in ['m5', 'wasm']
                 for backend, cache in [('auto', 'default'), ('auto', 'cache'), ('no-alloc', 'default'), ('bs58', 'default')]]
        output = tables(rows, sizes)
        self.assertIn('| 32 B | **100 ns** / **100 ns** | **100 ns** / **100 ns** | **100 ns** / **100 ns** | **100 ns** / **100 ns** | — / — |', output)
        self.assertIn('| Auto | 1.0 KiB / 2.0 KiB | 3.0 KiB / 4.0 KiB |', output)
        with self.assertRaisesRegex(ValueError, 'duplicate linked-size'):
            tables(rows, sizes + [sizes[0]])
        for row in rows:
            row.update(status='skipped', reason='not measured', warm_ns='',
                       process_min_ns='', process_max_ns='', primary_clock='')
        self.assertNotIn('**—**', tables(rows, sizes))


class BackendInputTests(unittest.TestCase):
    def row(self, **changes):
        row = dict(platform='gcp-generic', cache='default', backend='auto', operation='encode',
                   decoded_bytes='32', status='measured', reason='', warm_ns='100',
                   primary_clock='process CPU', warm_observations='9')
        row.update(changes)
        return row

    def test_gcp_schema_accepts_no_source_fingerprint(self):
        render_backends.validate_rows([self.row()])

    def test_rejects_invalid_measurements_before_output(self):
        for changes in [dict(warm_ns='nan'), dict(warm_ns='inf'), dict(warm_ns='0'),
                        dict(primary_clock='elapsed'), dict(warm_observations='3'),
                        dict(status='unknown'), dict(decoded_bytes='-1'), dict(platform='unknown')]:
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                render_backends.validate_rows([self.row(**changes)])

    def test_rejects_duplicates_and_mixed_sources(self):
        for rows in [[self.row(), self.row()],
                     [self.row(source_fingerprint='a' * 64),
                      self.row(decoded_bytes='64', source_fingerprint='b' * 64)]]:
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                render_backends.validate_rows(rows)

    def test_skips_require_reason_and_no_timing(self):
        for changes in [dict(status='skipped', reason=''),
                        dict(status='skipped', reason='policy cap'),
                        dict(status='skipped', reason='policy cap', warm_ns='',
                             primary_clock='', warm_observations='', warm_min_ns='5')]:
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                render_backends.validate_rows([self.row(**changes)])
        render_backends.validate_rows([self.row(status='skipped', reason='policy cap', warm_ns='',
                                               primary_clock='', warm_observations='')])

    def test_wasm_uses_elapsed_clock(self):
        render_backends.validate_rows([self.row(platform='wasm', primary_clock='elapsed')])
        with self.assertRaises(ValueError):
            render_backends.validate_rows([self.row(platform='wasm')])

    def test_invalid_csv_does_not_create_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            source, output = directory / 'input.csv', directory / 'output'
            row = self.row(warm_ns='nan')
            with source.open('w', newline='') as stream:
                writer = csv.DictWriter(stream, fieldnames=row)
                writer.writeheader()
                writer.writerow(row)
            result = subprocess.run([sys.executable, str(Path(__file__).with_name('render_backends.py')),
                                     '--data', str(source), '--output', str(output)],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Invalid backend input', result.stderr)
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
