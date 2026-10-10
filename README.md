# PDFTrimly

PDFTrimly is a desktop application under development for local, offline PDF compression
on Windows and Linux, developed by Pedro Paulo Reis.

**Status: engine and desktop interface in development (0.1.0).** The command-line
executable optimizes streams and eligible scan images, preserves existing OCR,
validates the rewritten structure and reports measured savings. The desktop
UI is available for Linux with multiple-file selection and sequential batch
processing, PDF/folder drag-and-drop and five embedded interface/CLI languages.
Settings persist language, System/Light/Dark theme, compression level and output
folder and recursive discovery defaults. Folder inputs preserve relative
subdirectories, exclude the destination and deduplicate paths. A compact list
keeps full paths and results in a separate detail panel. Results can be copied and the output folder opened.
Installers are a future milestone.

## Project structure

- `src/main.rs`: application entry point.
- `crates/pdftrimly-core`: independent engine, stream optimization and validation.
- `assets/logo.svg`: original project logo.
- `i18n/`: embedded English, Brazilian Portuguese, French, Spanish and Italian catalogs.
- `src/gui_state.rs`: language-independent batch state and channel handling.
- `src/config.rs` and `src/settings.rs`: versioned preferences and settings flow.
- `src/desktop.rs` and `src/actions.rs`: clipboard, folder and product links.
- `.github/workflows/ci.yml`: Linux and Windows compilation and checks.
- `scripts/build-local.sh`: local release build helper.
- `docs/`: architecture, dependency decisions and implementation stages.
- `projeto.md`: full development specification in Portuguese.

The application depends on the core crate. The core must remain independent
of the UI, filesystem dialogs and application settings.

## Build and run

Install Rust stable with Cargo, rustfmt and Clippy using
[rustup](https://rustup.rs/). From the repository root:

```sh
cargo build --workspace --locked
cargo run --locked -- --version
cargo run --locked -- --help
cargo run --locked # open desktop interface
cargo run --locked -- compress "input.pdf" "output.pdf" --level normal
```

Saved preferences select startup defaults. On first startup, the system locale
selects the initial language with English fallback.
`PDFTRIMLY_LANG=pt-BR cargo run --locked` explicitly selects a language.
The main-window choice lasts for the session; Settings saves startup defaults.
See the [folder input guide](docs/folder-input.md) for discovery rules.
See the [settings guide](docs/settings.md) for storage and recovery.

On Linux, `bash scripts/build-local.sh` builds the release executable.
No installers or published releases are available at this stage. See the
[Linux interface guide](docs/linux-interface.md) for operation and limitations.

## Development checks

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p pdftrimly-core --locked
```

Integration tests exercise real PDF files, original preservation, searchable
text, shared page content, existing outputs, encryption, signature fields,
resource limits and unsupported stream filters. See the
[proof-of-concept report](docs/proof-of-concept.md) for results and limits.

## Roadmap and constraints

Light preserves image data; Normal (default) targets 150 effective DPI and
JPEG quality 80; Ultra targets 96 DPI and quality 60. Normal and Ultra change
image quality. Text, including invisible OCR, remains intact. Unsupported
images, masks and complex color spaces are preserved. See
[image optimization results](docs/image-optimization.md).

The next milestone is complete graphical verification and broader validation
with representative real-world PDFs. In-file cancellation and release
packaging are still pending. See the
[first release checklist](docs/first-release.md) and [language guide](docs/i18n.md).
See [architecture](docs/architecture.md)
and the complete [specification](projeto.md).

Processing must remain offline and must never modify the original PDF.
Encrypted and digitally signed documents require explicit handling. The
engine must report unchanged or larger results honestly and preserve
searchable text and supported document features.

## License and dependencies

Project-authored source code is licensed under MIT. The engine uses `lopdf`,
`flate2`, `image`, `tempfile` and `thiserror`. The desktop interface uses Slint and rfd. See
[dependency decisions](docs/dependencies.md) for license metadata and the
remaining distribution review.

Contributions should follow the staged specification and include checks
appropriate to the behavior changed. Never commit private document fixtures.

[Website](https://dev.pedropaulo.net/pdftrimly) ·
[Support on Ko-fi](https://ko-fi.com/pedropaulo)
