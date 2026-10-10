"""Reproduce synthetic metrics and independent rendering checks.

Requires Cargo and pdftoppm (Poppler). Poppler is a verification tool only.
"""

import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    root = Path(__file__).resolve().parent.parent
    if not shutil.which("cargo") or not shutil.which("pdftoppm"):
        raise SystemExit("This verification requires cargo and pdftoppm in PATH.")
    subprocess.run(
        ["cargo", "build", "--workspace", "--locked"], cwd=root, check=True
    )
    binary = root / "target" / "debug" / "pdftrimly"
    if not binary.exists():
        binary = binary.with_suffix(".exe")
    with tempfile.TemporaryDirectory(prefix="pdftrimly-poc-") as directory:
        fixtures = Path(directory) / "fixtures"
        subprocess.run(
            ["cargo", "run", "-p", "pdftrimly-core", "--example",
             "generate_fixtures", "--locked", "--", str(fixtures)],
            cwd=root, check=True,
        )
        for name in ["text", "vectors", "synthetic-scan", "mixed", "already-compressed"]:
            source = fixtures / f"{name}.pdf"
            destination = fixtures / f"{name}.optimized.pdf"
            original_hash = hashlib.sha256(source.read_bytes()).digest()
            result = subprocess.run(
                [str(binary), "compress", str(source), str(destination), "--level", "light"],
                cwd=root, check=True, capture_output=True, text=True, timeout=30,
            )
            print(f"\n{name}:\n{result.stdout}", end="")
            if hashlib.sha256(source.read_bytes()).digest() != original_hash:
                raise SystemExit(f"Original changed: {name}")
            rendered = [subprocess.run(
                ["pdftoppm", "-r", "72", "-singlefile", str(path)],
                check=True, capture_output=True, timeout=30,
            ).stdout for path in [source, destination]]
            if not rendered[0] or rendered[0] != rendered[1]:
                raise SystemExit(f"Rendering mismatch: {name}")
            print("Independent rendering: identical pixels at 72 DPI")


if __name__ == "__main__":
    main()
