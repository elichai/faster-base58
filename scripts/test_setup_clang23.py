#!/usr/bin/env python3
import json
from pathlib import Path
import unittest

import setup_clang23


LIBXML2 = {
    'Package': 'libxml2',
    'Version': '2.9.14+dfsg-1.3~deb12u6',
    'Filename': 'pool/main/libx/libxml2/libxml2_2.9.14+dfsg-1.3~deb12u6_amd64.deb',
    'Size': '688760',
    'SHA256': '4460e39dda10a815881374217cde08474747cfa018358cd8612c14b390eff53b',
    'URL': 'https://deb.debian.org/debian/pool/main/libx/libxml2/libxml2_2.9.14+dfsg-1.3~deb12u6_amd64.deb',
}

LIBICU72 = {
    'Package': 'libicu72',
    'Version': '72.1-3+deb12u1',
    'Filename': 'pool/main/i/icu/libicu72_72.1-3+deb12u1_amd64.deb',
    'Size': '9376124',
    'SHA256': 'f7f6f99c6d7b025914df2447fc93e11d22c44c0c8bdd8b6f36691c9e7ddcef88',
    'URL': 'https://deb.debian.org/debian/pool/main/i/icu/libicu72_72.1-3+deb12u1_amd64.deb',
}


class ClangPackageTests(unittest.TestCase):
    def test_manifest_includes_the_pinned_runtime_abis(self):
        manifest = json.loads(
            Path(setup_clang23.__file__).with_name('clang23-packages.json').read_text())
        for package in [LIBXML2, LIBICU72]:
            self.assertIn(package, manifest['packages'])
            setup_clang23.validate_package(package)


if __name__ == '__main__':
    unittest.main()
