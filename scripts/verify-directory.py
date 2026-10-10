"""Validate a local PDF collection without recording document text.

Usage: python3 scripts/verify-directory.py <input-directory> [light|normal|ultra]
Requires the release binary and Poppler's pdfinfo, pdftotext and pdftoppm.
Outputs and JSON results go to a fresh directory under dist/validation/.
"""

import datetime
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(chunk)
    return checksum.hexdigest()


def run(args, timeout=60):
    result = subprocess.run(args, capture_output=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"{Path(args[0]).name} exited with code {result.returncode}")
    return result.stdout


def pages(path):
    info = run(["pdfinfo", str(path)]).decode("utf-8", errors="replace")
    match = re.search(r"^Pages:\s+(\d+)$", info, re.MULTILINE)
    if not match:
        raise RuntimeError("pdfinfo did not report a page count")
    return int(match.group(1))


def main():
    root = Path(__file__).resolve().parent.parent
    if len(sys.argv) not in [2, 3]:
        raise SystemExit("Usage: verify-directory.py <input-directory> [light|normal|ultra]")
    level = sys.argv[2] if len(sys.argv) == 3 else "normal"
    if level not in ["light", "normal", "ultra"]:
        raise SystemExit("Invalid compression level")
    directory = Path(sys.argv[1]).resolve()
    if not directory.is_dir():
        raise SystemExit("The input directory does not exist")
    for tool in ["pdfinfo", "pdftotext", "pdftoppm"]:
        if not shutil.which(tool):
            raise SystemExit(f"Required verification tool missing: {tool}")
    binary = root / "target" / "release" / "pdftrimly"
    if not binary.exists():
        raise SystemExit("Build the release executable first: cargo build --release --locked")
    files = sorted(path for path in directory.rglob("*") if path.is_file() and path.suffix.lower() == ".pdf")
    if not files:
        raise SystemExit("No PDFs found")
    base = root / "dist" / "validation"
    base.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=base))
    results = []
    for source in files:
        relative = source.relative_to(directory)
        destination = output / "pdfs" / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        record = {"file": str(relative), "input_bytes": source.stat().st_size, "level": level,
                  "output": str(destination), "render_dpi": 72}
        before = digest(source)
        started = time.monotonic()
        print(f"Testing {relative} ({record['input_bytes']} bytes)", flush=True)
        try:
            result = subprocess.run(
                [str(binary), "compress", str(source), str(destination), "--level", level],
                capture_output=True, text=True, timeout=60,
            )
            record["engine_exit_code"] = result.returncode
            record["engine_stdout"] = result.stdout
            record["engine_stderr"] = result.stderr
            if result.returncode:
                record["status"] = "engine_rejected"
            else:
                record["output_bytes"] = destination.stat().st_size
                record["savings_percent"] = 100 * (1 - record["output_bytes"] / record["input_bytes"])
                record["input_pages"] = pages(source)
                record["output_pages"] = pages(destination)
                if record["input_pages"] != record["output_pages"]:
                    raise RuntimeError("Page count mismatch")
                text_before = run(["pdftotext", "-layout", str(source), "-"])
                text_after = run(["pdftotext", "-layout", str(destination), "-"])
                record["extracted_text_bytes"] = len(text_before)
                record["has_nonwhitespace_text"] = bool(text_before.strip())
                record["text_identical"] = text_before == text_after
                if not record["text_identical"]:
                    raise RuntimeError("Extracted text mismatch")
                del text_before, text_after
                mismatches = []
                mean_pixel_errors = []
                for page in range(1, record["input_pages"] + 1):
                    images = [run([
                        "pdftoppm", "-r", "72", "-f", str(page), "-l", str(page),
                        "-singlefile", str(path),
                    ]) for path in [source, destination]]
                    if not images[0] or images[0] != images[1]:
                        mismatches.append(page)
                    # pdftoppm emits raw P6 RGB with three header lines here.
                    headers = [data.split(b"\n", 3) for data in images]
                    if any(len(header) != 4 or header[0] != b"P6" or header[2] != b"255" for header in headers):
                        raise RuntimeError("Unexpected raster format")
                    if headers[0][:3] != headers[1][:3] or len(headers[0][3]) != len(headers[1][3]):
                        raise RuntimeError("Rendered page dimensions changed")
                    mean_pixel_errors.append(round(sum(abs(a-b) for a,b in zip(headers[0][3], headers[1][3])) / len(headers[0][3]), 4))
                    print(f"  Render comparison: page {page}/{record['input_pages']}", flush=True)
                record["render_mismatches"] = mismatches
                record["all_pages_pixel_identical"] = not mismatches
                record["mean_absolute_rgb_error_per_page"] = mean_pixel_errors
                record["status"] = "passed" if not mismatches else (
                    "render_mismatch" if level == "light" else "structure_passed_visual_review_required")
        except (RuntimeError, OSError, subprocess.TimeoutExpired) as error:
            record["status"] = "verification_failed"
            record["error"] = str(error)
        finally:
            record["original_unchanged"] = before == digest(source)
            record["duration_seconds"] = round(time.monotonic() - started, 3)
            if not record["original_unchanged"]:
                record["status"] = "original_changed"
            results.append(record)
            print(f"  Result: {record['status']}", flush=True)
            report = {"created_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                      "input_directory": str(directory), "results": results}
            (output / "report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"Report: {output / 'report.json'}", flush=True)
    if any(result["status"] not in ["passed", "structure_passed_visual_review_required"] for result in results):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
