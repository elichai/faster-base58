"""Standard-library checks for the retained documentation and external inputs."""
import csv
import tempfile
import subprocess
import sys
import unittest
from pathlib import Path

from current import SERIES, TABLE_SIZES, check, size_deltas, tables, timing_cells
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
        names = ['m5-public', 'gcp-public', 'wasm-public']
        for name in names:
            (self.root / f'assets/graphs/{name}.svg').write_text(
                '<svg xmlns="http://www.w3.org/2000/svg"><title>Example</title></svg>')
        (self.root / 'README.md').write_text('\n'.join(f'![figure](assets/graphs/{name}.svg)' for name in names))

    def test_checks_retained_documents_without_raw_measurements(self):
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
        guide.write_text(guide.read_text().replace('![figure](assets/graphs/gcp-public.svg)', ''))
        with self.assertRaisesRegex(ValueError, 'README.*gcp-public'):
            check(self.root)

    def test_reference_and_encoded_links_are_checked(self):
        (self.root / 'DESIGN.md').write_text('[reference][target]\n\n[target]: docs/missing%20file.md')
        with self.assertRaisesRegex(ValueError, 'missing file.md'):
            check(self.root)


class ExternalInputTests(unittest.TestCase):
    def rows(self, platforms=('m5', 'wasm')):
        return [dict(platform=platform, cache=cache, backend=backend, operation=operation,
                     decoded_bytes=str(size), warm_ns='100', process_min_ns='90', process_max_ns='110',
                     clock='elapsed' if platform == 'wasm' else 'process CPU',
                     measurement_date='2026-11-05', source_commit='a' * 40)
                for platform in platforms for backend, cache, _ in SERIES
                for operation in ['encode', 'decode'] for size in TABLE_SIZES
                if backend != 'five8' or size in (32, 64)]

    def sizes(self, directory, values):
        """A sizes.csv with an empty probe and BACKEND: (code, readonly, wasm code, wasm data)."""
        path = Path(directory) / f'sizes-{len(list(Path(directory).iterdir()))}.csv'
        fields = ['backend', 'direction', 'kind', 'code_bytes', 'readonly_bytes',
                  'wasm_code_section_bytes', 'wasm_data_section_bytes']
        with path.open('w', newline='') as stream:
            writer = csv.writer(stream)
            writer.writerow(fields)
            for backend, sizes in {'empty': (1000, 1000, 1000, 1000), **values}.items():
                writer.writerow([backend, 'both', 'cdylib', *sizes])
        return path

    def test_tables_bold_minima_and_dash_unmeasured_cells(self):
        rows = self.rows()
        next(row for row in rows if row['backend'] == 'bs58' and row['decoded_bytes'] == '32').update(
            warm_ns='50', process_min_ns='40')
        with tempfile.TemporaryDirectory() as directory:
            deltas = size_deltas({'m5': self.sizes(directory, {'auto': (2024, 3048, 0, 0)}),
                                  'wasm': self.sizes(directory, {'auto': (0, 0, 4072, 5096)})})
        output = tables(rows, deltas)
        self.assertIn('### WebAssembly on Apple M5 Pro (Node/V8)', output)
        self.assertNotIn('AMD EPYC', output)
        self.assertIn('| 32 B | 100 ns / **100 ns** | 100 ns / **100 ns** | 100 ns / **100 ns** | **50 ns** / **100 ns** | 100 ns / **100 ns** |', output)
        self.assertIn('| 512 B | **100 ns** / **100 ns** | **100 ns** / **100 ns** | **100 ns** / **100 ns** | **100 ns** / **100 ns** | — / — |', output)
        self.assertIn('| Auto | 1.0 KiB / 2.0 KiB | 3.0 KiB / 4.0 KiB |', output)
        self.assertIn('| bs58 | — | — |', output)

    def test_size_cohorts_need_one_empty_probe(self):
        with tempfile.TemporaryDirectory() as directory:
            path = self.sizes(directory, {'auto': (1, 1, 1, 1)})
            path.write_text(path.read_text().replace('empty', 'auto'))
            with self.assertRaisesRegex(ValueError, 'empty'):
                size_deltas({'m5': path})
            with self.assertRaisesRegex(ValueError, 'cohort'):
                size_deltas({'arm': self.sizes(directory, {})})

    def test_invalid_rows_are_rejected(self):
        for changes in [dict(warm_ns='nan'), dict(process_max_ns='80'), dict(warm_ns=''),
                        dict(platform='unknown'), dict(clock='elapsed'),
                        dict(measurement_date='2026-10-29'), dict(source_commit='b' * 40)]:
            with self.subTest(changes=changes):
                rows = self.rows()
                rows[0].update(changes)
                with self.assertRaises(ValueError):
                    timing_cells(rows)

    def test_duplicate_measurements_are_rejected(self):
        rows = self.rows()
        with self.assertRaisesRegex(ValueError, 'Duplicate timing'):
            timing_cells(rows + [rows[0]])

    def test_renderer_requires_explicit_inputs(self):
        result = subprocess.run([sys.executable, str(Path(__file__).with_name('render_current.py')), '--help'],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('--data', result.stdout)
        self.assertIn('--sizes', result.stdout)


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
