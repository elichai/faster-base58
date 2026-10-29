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


class ClangPackageTests(unittest.TestCase):
    def test_manifest_includes_the_pinned_libxml2_abi(self):
        manifest = json.loads(
            Path(setup_clang23.__file__).with_name('clang23-packages.json').read_text())
        self.assertIn(LIBXML2, manifest['packages'])
        setup_clang23.validate_package(LIBXML2)


if __name__ == '__main__':
    unittest.main()
