# Dependency and license decisions

Reviewed on 2026-10-11 for release preparation 0.1.1. The project-authored code uses MIT. The engine uses
the following resolved direct dependencies, verified in Cargo metadata:

| Crate | Resolved version | Declared license |
| --- | --- | --- |
| image | 0.25.10 | MIT OR Apache-2.0 |
| lopdf | 0.45.0 | MIT |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| tempfile | 3.27.0 | MIT OR Apache-2.0 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |

`lopdf` is integrated without its optional default features; image embedding,
font embedding, async loading and parallel processing are disabled. `flate2`
uses its default Rust backend, without an external compression executable.
`image` enables only the JPEG feature for Rust decoding and encoding.
`Cargo.lock` records the resolved dependencies. License metadata for all
resolved packages was inspected, including alternatives and additional BSD,
Unicode and Zlib terms. The package script collects texts and notices; the distribution review below records what was actually checked and what remains.

## PDF engine candidates

The [lopdf documentation](https://docs.rs/lopdf/latest/lopdf/) identifies
the library as MIT-licensed, with an exception for its bundled Montserrat
font. No bundled font is copied by this project. The library is used only for
PDF parsing and rewriting; image and font embedding are disabled.

Parsing and rewriting PDF objects is not rendering or a guarantee of visual
equivalence. The proof of concept rejects encryption and signature fields,
checks document structures, and preserves unsupported image codecs and
filters without interpreting them. Eligible JPEG/Flate/raw RGB and gray images
can be recompressed using the Rust image crate. Broader feature coverage remains pending
real-world fixtures. Additional codecs will be reviewed when introduced.

## Desktop UI

[Slint's license terms](https://slint.dev/terms-and-conditions) offer GPLv3
and a royalty-free desktop/mobile/web license. Slint 1.18.1 is now integrated with Winit and software rendering.
rfd 0.15.4 (MIT) supplies Linux portal dialogs. Fontique 0.11.1
(Apache-2.0 OR MIT) enables dynamic Fontconfig loading on Linux.
Slint is not MIT-licensed
merely because application-authored source files use MIT.

The [Slint license overview](https://github.com/slint-ui/slint/blob/master/LICENSE.md)
states that the royalty-free route requires disclosure of Slint use. The selected route is the Royalty-free Desktop, Mobile, and Web Applications License 2.0. The GUI includes the AboutSlint widget as attribution. The desktop build
uses the royalty-free desktop license route; future redistribution must
include applicable dependency notices and license texts.

## Locale and catalogs

The application also directly uses `sys-locale` 0.3.2 and `toml`
1.1.8+spec-1.1.0, both declared MIT OR Apache-2.0 in their resolved crate
metadata. These packages were already present in `Cargo.lock`; the new direct
dependencies reuse them. Locale lookup and catalog parsing happen locally.

## Preferences and desktop actions

`tempfile` 3.27.0 is now also a direct application dependency for atomic
preference replacement. `arboard` 3.6.1 supplies clipboard text with
`wayland-data-control`, without image clipboard support. `webbrowser` 1.2.4
opens the two fixed product URLs. All three declare MIT OR Apache-2.0 in their
resolved metadata and were already present in the lockfile. Folder opening
uses the OS desktop launcher (`xdg-open` / `explorer.exe`), outside the engine.
These launchers are never used for PDF compression. OS configuration paths
are resolved from XDG_CONFIG_HOME/HOME on Linux and APPDATA on Windows; no new
native build dependency was introduced.

## Eventos de drop nativo

A feature `unstable-winit-030` de Slint 1.18.1 expõe o filtro de eventos
Winit para conectar `DroppedFile` à descoberta existente. Não há atualização
de versão do Slint; `Cargo.lock` foi regenerado offline para a feature.
No Linux com `DISPLAY`, a janela usa X11/XWayland, pois o Winit 0.30 não
implementa drop de arquivos em Wayland. Sem XWayland, use os seletores.

## Distribution review — 2026-10-11

Executed locally: `cargo metadata --offline --locked --filter-platform TARGET`
and `cargo tree --offline --locked --target TARGET --prefix none --format '{p}'`
for Linux GNU x86_64 and Windows MSVC x86_64, followed by `notices()` from
`scripts/package.py`. Windows dependency inspection on Linux is **not** a
Windows build or execution check. Inventory includes active build/test
packages, so it intentionally exceeds the code shipped in the executable.

| Target | External packages inspected | Missing recognized license files after supplements |
| --- | ---: | ---: |
| Linux GNU x86_64 | 370 | 0 |
| Windows MSVC x86_64 | 283 | 0 |

The crate archives omitted license texts for `alloc-stdlib` 0.2.4,
`lyon_algorithms` 1.0.21, `lyon_extra` 1.1.0, `lyon_geom`/`lyon_path` 1.0.19,
`rspolib` 0.1.2, `taffy` 0.10.1 and Windows `clipboard-win` 5.4.1.
Supplemental upstream texts are versioned in `assets/licenses/supplemental/`.
Each `PROVENANCE.json` records a URL pinned to the commit from the published
crate's `.cargo_vcs_info.json`, retrieval date and SHA-256. Packaging verifies
these hashes and includes provenance with the text. Lyon uses the MIT option
of `MIT OR Apache-2.0`; alloc-stdlib is BSD-3-Clause, rspolib and taffy are MIT,
and clipboard-win is BSL-1.0. The texts were downloaded, read and retained
without substituting a generic license template.

The collector also copies texts inside `LICENSES/`, including Slint's bundled
Royalty-free 2.0 terms. The source contains `AboutSlint` in the top-level About
dialog, consistent with the attribution route described in the
[official Slint terms](https://slint.dev/terms-and-conditions).
Native visibility/clickability of that dialog remains part of the clean-VM test.

A zero missing-file count means files were found, not that every obligation
or embedded component was approved. Before public binary distribution:

- Review every selected license/NOTICE and embedded data notice against the
  actual release binary, including fonts and generated code. Build-only crates
  do not automatically impose runtime redistribution obligations.
- Complete the AppImage runtime review. The existing `appimage-runtime.txt`
  contains MIT terms and **links**, rather than complete texts, for statically
  linked musl, libfuse, squashfuse, zstd and zlib. Identify exact versions/source
  and applicable obligations from the pinned runtime's build provenance; bundle
  complete required notices and any required corresponding source/relinking
  materials. The [runtime repository](https://github.com/AppImage/type2-runtime)
  is the upstream starting point. This is an open release blocker.
- Inspect the AppImage's actual native library inventory and Debian copyright
  texts. The script now stops for unidentified libraries instead of silently
  omitting their notices. Verify source availability and replacement/relinking
  requirements for any LGPL components; copyright copying alone is insufficient.
- Execute [clean-install.md](clean-install.md) on the finished artifacts and
  preserve the evidence. No clean Linux/Windows VM install was executed here.

The workflow prepares drafts only. These remaining checks must be completed
before a human publishes a release; this revision neither publishes nor certifies
license compliance.
