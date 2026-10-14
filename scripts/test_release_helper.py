import pathlib
import os
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("release.sh").resolve()


class ReleaseHelperTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "Release Test")
        self.git("config", "user.email", "release-test@example.invalid")
        (self.root / "tracked.txt").write_text("base\n", encoding="utf-8", newline="\n")
        self.git("add", "tracked.txt")
        self.git("commit", "-qm", "initial")

    def tearDown(self):
        self.temp.cleanup()

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, check=True,
                              text=True, capture_output=True)

    def run_helper(self, answers, env=None):
        # Binary pipes avoid Windows translating LF input to CRLF for Bash.
        result = subprocess.run(self.script_command(), cwd=self.root,
                                input=answers.encode("utf-8"),
                                capture_output=True, env=env)
        result.stdout = result.stdout.decode("utf-8")
        result.stderr = result.stderr.decode("utf-8")
        return result

    def script_command(self):
        bash = shutil.which("bash")
        if bash is None:
            self.fail("Bash is required to run scripts/release.sh tests")
        return [bash, str(SCRIPT)]

    def test_script_command_uses_bash(self):
        self.assertEqual(self.script_command()[0], shutil.which("bash"))

    def test_empty_index_refuses_commit(self):
        result = self.run_helper("1\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Não há alterações staged", result.stderr)

    def test_unstaged_file_refuses_before_menu(self):
        (self.root / "tracked.txt").write_text("unstaged\n", encoding="utf-8", newline="\n")
        result = self.run_helper("1\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("não staged", result.stderr)

    def test_untracked_file_refuses_before_menu(self):
        (self.root / "new.txt").write_text("untracked\n", encoding="utf-8", newline="\n")
        result = self.run_helper("1\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("não rastreados", result.stderr)

    def test_commit_uses_staged_change(self):
        (self.root / "tracked.txt").write_text("staged\n", encoding="utf-8", newline="\n")
        self.git("add", "tracked.txt")
        result = self.run_helper("1\nfeat: update\ns\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("+staged", result.stdout)
        self.assertEqual(self.git("log", "-1", "--pretty=%s").stdout.strip(),
                         "feat: update")

    def test_commit_and_push_branch_updates_only_the_local_test_remote(self):
        remote = self.root.parent / (self.root.name + "-push.git")
        self.addCleanup(shutil.rmtree, remote, ignore_errors=True)
        subprocess.run(["git", "init", "--bare", "-q", str(remote)], check=True)
        self.git("remote", "add", "origin", str(remote))
        self.git("push", "-qu", "origin", "main")
        (self.root / "tracked.txt").write_text("pushed\n", encoding="utf-8", newline="\n")
        self.git("add", "tracked.txt")
        result = self.run_helper("2\nfeat: push\ns\ns\n")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        remote_head = subprocess.run(
            ["git", "--git-dir", str(remote), "log", "-1", "--pretty=%s", "main"],
            check=True, capture_output=True, text=True,
        )
        self.assertEqual(remote_head.stdout.strip(), "feat: push")

    def test_declining_commit_does_not_create_commit(self):
        (self.root / "tracked.txt").write_text("staged\n", encoding="utf-8", newline="\n")
        self.git("add", "tracked.txt")
        result = self.run_helper("1\nfeat: update\nn\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.git("log", "-1", "--pretty=%s").stdout.strip(),
                         "initial")

    def test_release_refuses_non_main_branch(self):
        self.git("checkout", "-qb", "feature")
        result = self.run_helper("3\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("branch main", result.stderr)

    def test_release_updates_prerelease_cargo_versions_before_tag_push(self):
        core = self.root / "crates" / "pdftrimly-core"
        core.mkdir(parents=True)
        (core / "Cargo.toml").write_text(
            '[package]\nname = "pdftrimly-core"\nversion.workspace = true\n'
            'edition = "2021"\n',
            encoding="utf-8", newline="\n",
        )
        (core / "src").mkdir()
        (core / "src" / "lib.rs").write_text("pub fn ready() {}\n", encoding="utf-8", newline="\n")
        (self.root / "Cargo.toml").write_text(
            '[workspace]\nmembers = ["crates/pdftrimly-core"]\nresolver = "2"\n\n'
            '[workspace.package]\nversion = "0.1.3-rc.1"\n\n'
            '[package]\nname = "pdftrimly"\nversion.workspace = true\n'
            'edition = "2021"\n\n[dependencies]\n'
            'pdftrimly-core = { path = "crates/pdftrimly-core" }\n',
            encoding="utf-8", newline="\n",
        )
        (self.root / ".gitignore").write_text("/target/\n", encoding="utf-8", newline="\n")
        (self.root / "src").mkdir()
        (self.root / "src" / "main.rs").write_text("fn main() {}\n", encoding="utf-8", newline="\n")
        subprocess.run(["cargo", "check", "--offline"], cwd=self.root,
                       check=True, capture_output=True, text=True)
        self.git("add", ".")
        self.git("commit", "-qm", "prepare release")
        remote = self.root.parent / (self.root.name + "-remote.git")
        self.addCleanup(shutil.rmtree, remote, ignore_errors=True)
        subprocess.run(["git", "init", "--bare", "-q", str(remote)], check=True)
        self.git("remote", "add", "origin", str(remote))
        self.git("push", "-qu", "origin", "main")

        fake_tmp = tempfile.TemporaryDirectory()
        self.addCleanup(fake_tmp.cleanup)
        fake_bin = pathlib.Path(fake_tmp.name)
        gh = fake_bin / "gh"
        gh.write_text("#!/bin/sh\n[ \"$1 $2\" = \"auth status\" ]\n", encoding="utf-8", newline="\n")
        gh.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = os.pathsep.join((str(fake_bin), env["PATH"]))
        result = self.run_helper("3\n0.1.3\ns\ns\ns\n", env=env)
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        self.assertIn('version = "0.1.3"', (self.root / "Cargo.toml").read_text())
        refs = subprocess.run(["git", "--git-dir", str(remote), "tag"],
                              check=True, capture_output=True, text=True)
        self.assertIn("v0.1.3", refs.stdout.splitlines())
        self.assertIn("prepare release", result.stdout)
        self.assertIn("initial", result.stdout)

    def test_release_refuses_existing_local_and_remote_tags(self):
        remote = self.root.parent / (self.root.name + "-tags.git")
        self.addCleanup(shutil.rmtree, remote, ignore_errors=True)
        subprocess.run(["git", "init", "--bare", "-q", str(remote)], check=True)
        self.git("remote", "add", "origin", str(remote))
        self.git("push", "-qu", "origin", "main")
        fake_tmp = tempfile.TemporaryDirectory()
        self.addCleanup(fake_tmp.cleanup)
        gh = pathlib.Path(fake_tmp.name) / "gh"
        gh.write_text("#!/bin/sh\n[ \"$1 $2\" = \"auth status\" ]\n", encoding="utf-8", newline="\n")
        gh.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = os.pathsep.join((fake_tmp.name, env["PATH"]))

        self.git("tag", "v0.1.3")
        local = self.run_helper("3\n0.1.3\n", env=env)
        self.assertNotEqual(local.returncode, 0)
        self.assertIn("A tag local", local.stderr)
        self.git("tag", "-d", "v0.1.3")

        self.git("tag", "v0.1.3")
        self.git("push", "origin", "v0.1.3")
        self.git("tag", "-d", "v0.1.3")
        remote_tag = self.run_helper("3\n0.1.3\n", env=env)
        self.assertNotEqual(remote_tag.returncode, 0)
        self.assertIn("A tag remota", remote_tag.stderr)

    def test_release_preflight_failure_never_commits_or_pushes(self):
        remote = self.root.parent / (self.root.name + "-preflight.git")
        self.addCleanup(shutil.rmtree, remote, ignore_errors=True)
        subprocess.run(["git", "init", "--bare", "-q", str(remote)], check=True)
        self.git("remote", "add", "origin", str(remote))
        self.git("push", "-qu", "origin", "main")
        (self.root / "Cargo.toml").write_text(
            '[workspace.package]\nversion = "0.1.1"\n',
            encoding="utf-8", newline="\n",
        )
        (self.root / "Cargo.lock").write_text("# synthetic lockfile\n", encoding="utf-8", newline="\n")
        self.git("add", "Cargo.toml", "Cargo.lock")
        self.git("commit", "-qm", "add workspace metadata")
        self.git("push", "origin", "main")

        fake_tmp = tempfile.TemporaryDirectory()
        self.addCleanup(fake_tmp.cleanup)
        fake_bin = pathlib.Path(fake_tmp.name)
        gh = fake_bin / "gh"
        gh.write_text("#!/bin/sh\n[ \"$1 $2\" = \"auth status\" ]\n", encoding="utf-8", newline="\n")
        gh.chmod(0o755)
        cargo = fake_bin / "cargo"
        cargo.write_text("#!/bin/sh\nprintf 'preflight failed\\n' >&2\nexit 1\n", encoding="utf-8", newline="\n")
        cargo.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = os.pathsep.join((str(fake_bin), env["PATH"]))
        before = self.git("rev-parse", "HEAD").stdout.strip()
        result = self.run_helper("3\n0.1.2\ns\n", env=env)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("preflight failed", result.stderr)
        self.assertEqual(self.git("rev-parse", "HEAD").stdout.strip(), before)
        self.assertFalse(self.git("tag", "--list", "v0.1.2").stdout.strip())
        remote_head = subprocess.run(
            ["git", "--git-dir", str(remote), "rev-parse", "refs/heads/main"],
            check=True, capture_output=True, text=True,
        )
        self.assertEqual(remote_head.stdout.strip(), before)

    def test_release_stops_when_github_cli_is_not_authenticated(self):
        fake_tmp = tempfile.TemporaryDirectory()
        self.addCleanup(fake_tmp.cleanup)
        gh = pathlib.Path(fake_tmp.name) / "gh"
        gh.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8", newline="\n")
        gh.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = os.pathsep.join((fake_tmp.name, env["PATH"]))
        result = self.run_helper("3\n", env=env)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Autentique", result.stderr)


if __name__ == "__main__":
    unittest.main()
