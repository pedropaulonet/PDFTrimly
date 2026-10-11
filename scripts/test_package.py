"""Regression checks for dependency notice collection (stdlib only)."""
import importlib.util
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('packaging_script', Path(__file__).with_name('package.py'))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class NoticesTests(unittest.TestCase):
    def test_supplement_hashes_survive_windows_style_git_checkout(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / 'source'
            source.mkdir()
            shutil.copytree(package.ROOT / 'assets/licenses/supplemental',
                            source / 'assets/licenses/supplemental')
            attributes = package.ROOT / '.gitattributes'
            if attributes.is_file():
                shutil.copyfile(attributes, source / '.gitattributes')
            def git(*args):
                subprocess.run(['git', *args], cwd=source, check=True, capture_output=True)
            git('init', '--quiet')
            git('-c', 'core.autocrlf=false', 'add', '.')
            git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                '-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'Fixture')
            checkout = root / 'checkout'
            git('-c', 'core.autocrlf=true', 'clone', '--quiet', '--no-local',
                str(source), str(checkout))
            for provenance in checkout.glob('assets/licenses/supplemental/*/PROVENANCE.json'):
                for filename, expected in json.loads(provenance.read_text(encoding='utf-8'))['files'].items():
                    self.assertEqual(hashlib.sha256((provenance.parent / filename).read_bytes()).hexdigest(),
                                     expected, str(provenance.parent.name))

    def test_missing_license_is_reported_and_inactive_crates_are_excluded(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'active').mkdir()
            (root / 'inactive').mkdir()
            metadata = {'workspace_members': [], 'packages': [
                {'id': name, 'name': name, 'version': '1.0.0', 'license': 'MIT',
                 'manifest_path': str(root / name / 'Cargo.toml')}
                for name in ('active', 'inactive')]}
            package.notices(metadata, root / 'notices', {('active', '1.0.0')})
            audit = json.loads((root / 'notices/NOTICE-AUDIT.json').read_text())
            self.assertEqual(audit['missing_license_files'], ['active-1.0.0'])
            self.assertEqual(audit['dependency_count'], 1)
            self.assertNotIn('inactive-1.0.0', (root / 'notices/THIRD-PARTY-NOTICES.txt').read_text())

    def test_nested_notice_and_declared_license_file_are_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'crate/LICENSES').mkdir(parents=True)
            (root / 'crate/LICENSES/MIT.txt').write_text('upstream copyright')
            (root / 'crate/NOTICE').write_text('required attribution')
            metadata = {'workspace_members': [], 'packages': [{
                'id': 'crate', 'name': 'crate', 'version': '1.0.0', 'license': 'MIT',
                'license_file': 'LICENSES/MIT.txt', 'manifest_path': str(root / 'crate/Cargo.toml')}]}
            package.notices(metadata, root / 'notices', {('crate', '1.0.0')})
            self.assertEqual((root / 'notices/crate-1.0.0/NOTICE').read_text(), 'required attribution')
            self.assertEqual((root / 'notices/crate-1.0.0/LICENSES/MIT.txt').read_text(), 'upstream copyright')
            self.assertEqual(json.loads((root / 'notices/NOTICE-AUDIT.json').read_text())['missing_license_files'], [])

    def test_pinned_supplement_completes_license_omitted_from_crate(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            metadata = {'workspace_members': [], 'packages': [{
                'id': 'alloc', 'name': 'alloc-stdlib', 'version': '0.2.4',
                'license': 'BSD-3-Clause', 'manifest_path': str(root / 'Cargo.toml')}]}
            package.notices(metadata, root / 'notices', {('alloc-stdlib', '0.2.4')})
            audit = json.loads((root / 'notices/NOTICE-AUDIT.json').read_text())
            self.assertEqual(audit['missing_license_files'], [])
            self.assertIn('Dropbox', (root / 'notices/alloc-stdlib-0.2.4/LICENSE').read_text())
            self.assertTrue((root / 'notices/alloc-stdlib-0.2.4/PROVENANCE.json').is_file())


if __name__ == '__main__':
    unittest.main()
