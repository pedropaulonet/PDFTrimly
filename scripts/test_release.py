"""Exercise the workflow release shell with a local gh stub; no network calls."""
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest


@unittest.skipUnless(os.name == 'posix', 'Release job uses Linux/bash')
class ReleaseTests(unittest.TestCase):
    def test_existing_release_receives_assets_and_is_published(self):
        self.check_release(existing=True)

    def test_new_release_is_created_as_draft_before_assets_and_publication(self):
        self.check_release(existing=False)

    def check_release(self, existing):
        workflow = (Path(__file__).resolve().parents[1] / '.github/workflows/packages.yml').read_text()
        shell = textwrap.dedent(workflow.rsplit('        run: |\n', 1)[1])
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'release').mkdir()
            (root / 'release/package.deb').write_text('fixture')
            stub = root / 'gh'
            stub.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >> "$GH_TEST_LOG"\n'
                            'if [ "$2" = view ]; then\n'
                            '  if [ "$GH_TEST_EXISTS" = 1 ]; then echo false; else exit 1; fi\n'
                            'fi\n')
            stub.chmod(0o755)
            log = root / 'calls'
            result = subprocess.run(['bash', '-e', '-c', shell], cwd=root,
                env=dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH'],
                         RELEASE_TAG='v0.1.1', GH_TEST_LOG=str(log),
                         GH_TEST_EXISTS='1' if existing else '0'), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            calls = log.read_text().splitlines()
            self.assertTrue(any(c.startswith('release upload v0.1.1 ') for c in calls), calls)
            self.assertEqual(calls[-1], 'release edit v0.1.1 --draft=false --prerelease=true')
            creates = [c for c in calls if c.startswith('release create ')]
            self.assertEqual(len(creates), 0 if existing else 1)
            if creates:
                self.assertIn('--draft', creates[0])
                self.assertLess(calls.index(creates[0]), next(i for i,c in enumerate(calls) if c.startswith('release upload ')))
