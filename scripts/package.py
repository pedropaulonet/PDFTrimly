#!/usr/bin/env python3
"""Package a built PDFTrimly executable; never install packages on the host."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
# Fontique, Winit and arboard load these libraries at runtime, outside ELF NEEDED.
RUNTIME_LIBRARIES = {
    "libfontconfig.so.1": "libfontconfig1",
    "libX11.so.6": "libx11-6",
    "libX11-xcb.so.1": "libx11-xcb1",
    "libXcursor.so.1": "libxcursor1",
    "libXrandr.so.2": "libxrandr2",
    "libXi.so.6": "libxi6",
    "libXfixes.so.3": "libxfixes3",
    "libXrender.so.1": "libxrender1",
    "libxcb.so.1": "libxcb1",
    "libwayland-client.so.0": "libwayland-client0",
    "libwayland-cursor.so.0": "libwayland-cursor0",
    "libwayland-egl.so.1": "libwayland-egl1",
    "libxkbcommon.so.0": "libxkbcommon0",
    "libxkbcommon-x11.so.0": "libxkbcommon-x11-0",
}
LINUXDEPLOY_SHA256 = "c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d"
RUNTIME_SHA256 = "156f4bdbde9c52d01814600013e0a273f0118dc2de98975f3c8c63427ec79074"


def run(*args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def copy(source, target, executable=False):
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    target.chmod(0o755 if executable else 0o644)


def notices(metadata, destination):
    destination.mkdir(parents=True, exist_ok=True)
    copy(ROOT / "assets/licenses/appimage-runtime.txt", destination / "appimage-runtime.txt")
    lines = ["PDFTrimly third-party dependencies", "",
             "Includes build dependencies. License declarations are from Cargo metadata.",
             "Slint: Royalty-free Desktop, Mobile and Web Applications License.",
             "The GUI includes AboutSlint attribution. See the bundled Slint license texts.", ""]
    workspace = set(metadata["workspace_members"])
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if package["id"] in workspace:
            continue
        base = Path(package["manifest_path"]).parent
        name = f'{package["name"]}-{package["version"]}'
        lines.extend([name, f'License: {package.get("license") or "See license files"}',
                      f'Source: {package.get("repository") or package.get("source") or "Cargo registry"}'])
        candidates = set()
        for path in base.rglob("*"):
            relative = path.relative_to(base)
            if len(relative.parts) > 3 or not path.is_file():
                continue
            if path.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE")):
                candidates.add(path)
        if package.get("license_file"):
            candidates.add(base / package["license_file"])
        for path in sorted(candidates):
            copy(path, destination / name / path.relative_to(base))
        if not candidates:
            lines.append("No separate license file included in the crate archive.")
        lines.append("")
    (destination / "THIRD-PARTY-NOTICES.txt").write_text("\n".join(lines), encoding="utf-8")


def install_tree(binary, tree, license_dir):
    copy(binary, tree / "usr/bin/pdftrimly", executable=True)
    copy(ROOT / "assets/pdftrimly.desktop", tree / "usr/share/applications/pdftrimly.desktop")
    copy(ROOT / "assets/logo.svg", tree / "usr/share/icons/hicolor/scalable/apps/pdftrimly.svg")
    copy(ROOT / "LICENSE", tree / "usr/share/doc/pdftrimly/copyright")
    shutil.copytree(license_dir, tree / "usr/share/doc/pdftrimly/licenses")


def deb(binary, version, output, license_dir):
    with tempfile.TemporaryDirectory(prefix="pdftrimly-deb-") as temporary:
        work = Path(temporary)
        tree = work / "package"
        install_tree(binary, tree, license_dir)
        # dpkg-shlibdeps determines the actual minimum libc/libgcc requirements.
        (work / "debian").mkdir()
        (work / "debian/control").write_text(
            "Source: pdftrimly\nSection: utils\nPriority: optional\n"
            "Maintainer: Pedro Paulo Reis <pedropaulo.mg@gmail.com>\n\n"
            "Package: pdftrimly\nArchitecture: amd64\nDescription: Offline PDF optimization\n",
            encoding="utf-8")
        linked = run("dpkg-shlibdeps", "-O", f"-e{binary}", cwd=work,
                     capture_output=True).stdout.strip()
        dependencies = linked.removeprefix("shlibs:Depends=")
        if not linked.startswith("shlibs:Depends=") or not dependencies:
            raise RuntimeError("Could not determine Debian shared-library dependencies")
        runtime = sorted(set(RUNTIME_LIBRARIES.values()) | {"xdg-utils", "fonts-dejavu-core"})
        (tree / "DEBIAN").mkdir()
        size = sum(p.stat().st_size for p in tree.rglob("*") if p.is_file()) // 1024 + 1
        (tree / "DEBIAN/control").write_text(
            f"Package: pdftrimly\nVersion: {version}\nArchitecture: amd64\n"
            "Maintainer: Pedro Paulo Reis <pedropaulo.mg@gmail.com>\n"
            f"Installed-Size: {size}\nSection: utils\nPriority: optional\n"
            f'Depends: {dependencies}, {", ".join(runtime)}\n'
            "Recommends: xdg-desktop-portal\n"
            "Homepage: https://dev.pedropaulo.net/pdftrimly\n"
            "Description: Local and offline PDF compression\n"
            " Optimize PDF streams and scan images while preserving existing OCR.\n",
            encoding="utf-8")
        package = output / f"pdftrimly_{version}_amd64.deb"
        run("dpkg-deb", "--root-owner-group", "--build", str(tree), str(package))
        run("dpkg-deb", "--info", str(package))
        extracted = work / "extracted"
        run("dpkg-deb", "--extract", str(package), str(extracted))
        run(str(extracted / "usr/bin/pdftrimly"), "--version")


def appimage(binary, version, output, license_dir, linuxdeploy, runtime):
    if hashlib.sha256(linuxdeploy.read_bytes()).hexdigest() != LINUXDEPLOY_SHA256:
        raise RuntimeError("linuxdeploy SHA-256 mismatch")
    if hashlib.sha256(runtime.read_bytes()).hexdigest() != RUNTIME_SHA256:
        raise RuntimeError("AppImage runtime SHA-256 mismatch")
    linuxdeploy.chmod(0o755)
    library_paths = {}
    for line in run("ldconfig", "-p", capture_output=True).stdout.splitlines():
        if "=>" in line and "x86-64" in line:
            name = line.split()[0]
            library_paths.setdefault(name, Path(line.split("=>", 1)[1].strip()))
    missing = set(RUNTIME_LIBRARIES) - set(library_paths)
    if missing:
        raise RuntimeError(f"Missing AppImage runtime libraries: {sorted(missing)}")
    with tempfile.TemporaryDirectory(prefix="pdftrimly-appimage-") as temporary:
        work = Path(temporary)
        tree = work / "PDFTrimly.AppDir"
        install_tree(binary, tree, license_dir)
        arguments = [str(linuxdeploy), "--appdir", str(tree),
                     "--desktop-file", str(tree / "usr/share/applications/pdftrimly.desktop"),
                     "--icon-file", str(tree / "usr/share/icons/hicolor/scalable/apps/pdftrimly.svg")]
        for name in RUNTIME_LIBRARIES:
            arguments.extend(["--library", str(library_paths[name])])
        # Capture Debian copyright files for libraries bundled by linuxdeploy.
        # Dependencies copied transitively are covered after deployment below.
        env = dict(os.environ, APPIMAGE_EXTRACT_AND_RUN="1", ARCH="x86_64",
                   VERSION=version, NO_STRIP="1")
        run(*arguments, cwd=work, env=env)
        system_notices = tree / "usr/share/doc/pdftrimly/system-libraries"
        system_notices.mkdir()
        for library in (tree / "usr/lib").glob("*.so*"):
            original = library_paths.get(library.name)
            if original is None:
                continue
            # Account for both pre-usrmerge and usrmerge package-file listings.
            owner = None
            paths = {str(original), str(original.resolve())}
            paths.update(p.removeprefix("/usr") for p in list(paths) if p.startswith("/usr/lib/"))
            for path in sorted(paths):
                result = subprocess.run(["dpkg-query", "-S", path], text=True, capture_output=True)
                if result.returncode == 0:
                    owner = result.stdout.split(": ", 1)[0].split(":", 1)[0]
                    break
            if owner is None:
                raise RuntimeError(f"Could not identify library owner: {library.name}")
            copyright_file = Path("/usr/share/doc") / owner / "copyright"
            if not copyright_file.is_file():
                raise RuntimeError(f"Missing system copyright for {library.name}")
            copy(copyright_file, system_notices / f"{owner}.copyright")
        image = output / f"PDFTrimly-{version}-x86_64.AppImage"
        env["LDAI_OUTPUT"] = str(image)
        env["LDAI_RUNTIME_FILE"] = str(runtime)
        env["LDAI_NO_APPSTREAM"] = "1"
        run(str(linuxdeploy), "--appdir", str(tree), "--output", "appimage", cwd=work, env=env)
        image.chmod(0o755)
        run(str(image), "--version", env=env)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("platform", choices=("linux", "windows"))
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--linuxdeploy", type=Path)
    parser.add_argument("--runtime", type=Path)
    parser.add_argument("--output", type=Path, default=ROOT / "dist/artifacts")
    args = parser.parse_args()
    metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
    package = next(p for p in metadata["packages"] if p["name"] == "pdftrimly")
    version = package["version"]
    suffix = ".exe" if args.platform == "windows" else ""
    binary = (args.binary or ROOT / f"target/release/pdftrimly{suffix}").resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    licenses = output / "licenses"
    if licenses.exists():
        raise RuntimeError("Use a fresh output directory for each packaging run")
    notices(metadata, licenses)
    copy(ROOT / "LICENSE", output / "LICENSE")
    if args.platform == "windows":
        copy(binary, output / "pdftrimly.exe", executable=True)
        run(str(output / "pdftrimly.exe"), "--version")
    else:
        deb(binary, version, output, licenses)
        if args.linuxdeploy is None or args.runtime is None:
            raise RuntimeError("--linuxdeploy and --runtime are required for Linux packaging")
        appimage(binary, version, output, licenses, args.linuxdeploy.resolve(strict=True),
                 args.runtime.resolve(strict=True))
    sums = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}"
            for p in sorted(output.iterdir()) if p.is_file() and p.name != "SHA256SUMS"]
    (output / "SHA256SUMS").write_text("\n".join(sums) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
