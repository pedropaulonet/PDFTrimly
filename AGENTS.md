# AGENTS.md

Guidance for AI coding agents and other contributors working in this repository.
Keep these instructions aligned with the actual code, `README.md`, the Cargo
workspace, and the focused documentation in `docs/`. More-specific `AGENTS.md`
files in a subdirectory take precedence for work in that subtree.

## Project at a glance

PDFTrimly is a local, offline desktop application and command-line tool for
optimizing PDF files on Linux and Windows. It is a Rust 2021 workspace with two
packages:

- `pdftrimly` is the desktop application, CLI, settings, localization, and batch
  orchestration.
- `pdftrimly-core` is the independent PDF optimization library. It must remain
  usable without depending on the GUI or application layer.

The application uses Slint for its interface. The engine uses `lopdf` for PDF
object processing and the Rust `image` crate for supported image operations.
The project aims to preserve existing searchable text and OCR. PDF rewriting is
not PDF rendering, and output visual equivalence is not guaranteed for every
document.

## Source map

- `src/main.rs`: application startup, command-line parsing, and CLI output.
- `src/app.rs`: desktop window setup, callbacks, and connection between the UI
  and application services.
- `src/gui_state.rs`: semantic UI state, batch state, and displayed results.
- `src/batch.rs`: sequential batch work, output-name reservation, and batch
  errors.
- `src/discovery.rs`, `src/inputs.rs`: PDF discovery and adapting picker/drop
  inputs to application work.
- `src/ui_tasks.rs`: asynchronous I/O results delivered to the Slint UI thread.
- `src/config.rs`, `src/settings.rs`: persisted configuration and settings
  drafts/application.
- `src/i18n.rs`, `src/messages.rs`, `i18n/`: locale handling, translated
  messages, and embedded TOML catalogs.
- `src/actions.rs`, `src/desktop.rs`: UI actions and operating-system desktop
  services.
- `ui/main.slint`: desktop interface and visual components.
- `crates/pdftrimly-core/src/optimizer.rs`: file-level optimization flow.
- `crates/pdftrimly-core/src/validation.rs`: structural validation and
  unsupported-document checks.
- `crates/pdftrimly-core/src/streams.rs`: PDF stream optimization.
- `crates/pdftrimly-core/src/image_usage.rs`, `images.rs`: image placement
  analysis and eligible image processing.
- `tests/`, `src/app_tests.rs`, and
  `crates/pdftrimly-core/tests/`: CLI, application, and engine tests.
- `scripts/`: packaging helpers and Python verification scripts.
- `.github/workflows/`: CI and platform package/release workflows.
- `docs/`: architecture, behavior, localization, packaging, and verification
  notes. Read the relevant guide before changing behavior it documents.

## Working rules

1. Read the relevant implementation, tests, and documentation before changing
   behavior. Prefer the existing boundaries and patterns over a broad rewrite.
2. Keep `pdftrimly-core` independent from Slint, UI callbacks, localization, and
   desktop services. Translate typed engine or batch errors at the application
   presentation boundary.
3. Keep handwritten Rust free of `unsafe`. The workspace forbids unsafe code in
   the engine and denies it in application code; do not weaken these lints to
   silence a diagnostic. Slint-generated code has narrowly scoped allowances.
4. Preserve the existing dependency choices and `Cargo.lock`. Do not update,
   add, or replace dependencies without a concrete need; review their license,
   target, feature, and packaging implications as described in
   `docs/dependencies.md`.
5. Keep changes focused. Do not reformat unrelated files, rewrite generated or
   vendored content, or make unrelated cleanup part of a feature change.
6. Do not claim a build, test, platform check, visual review, or release check
   passed unless it was actually run and its result was inspected. State any
   unrun or environment-dependent checks clearly.

## PDF safety and product behavior

- Process documents locally. Do not add network upload, telemetry, or remote
  processing to PDF handling without an explicit product requirement.
- Never overwrite an input PDF or an already existing output. Preserve the
  current temporary-output, validation, and finalization safeguards when
  changing file-writing code.
- Keep output containment checks in batch processing. Treat input paths,
  directory contents, PDF object graphs, metadata, and filenames as untrusted
  input; handle malformed or cyclic structures without panics or unbounded
  processing.
- Encrypted and digitally signed PDFs are rejected. Do not silently remove
  encryption or invalidate a signature to make processing succeed.
- Unsupported image encodings and filters must remain preserved when the engine
  cannot safely interpret them. Do not corrupt unrelated PDF objects or page
  drawing commands while optimizing eligible streams or images.
- `Light` preserves image data. `Normal` and `Ultra` may downsample or
  recompress eligible images and therefore can reduce visual quality. Keep the
  UI text, CLI behavior, and documentation honest about that tradeoff.
- Preserve searchable text and existing OCR where possible. Do not describe
  structural PDF rewriting as a guarantee of visual equivalence; inspect
  representative rendered output when a change could affect appearance.
- Batch jobs are sequential. Stopping a batch currently allows the active file
  to finish; do not describe or implement immediate cancellation unless the
  behavior is deliberately changed across engine, batch, UI, tests, and docs.
- Avoid logging document contents or sensitive paths unnecessarily. Never add
  private or user-provided PDFs to the repository. Use synthetic fixtures and
  temporary directories for tests.

## Rust and interface conventions

- Use stable Rust, edition 2021, and the minimal toolchain components declared
  in `rust-toolchain.toml`. Keep dependency-sensitive commands locked.
- Follow standard Rust formatting and idiomatic error handling. Prefer typed
  errors and explicit outcomes over string matching or panics in production
  paths.
- Keep engine code deterministic and isolated from GUI concerns. Keep blocking
  file and PDF work off the UI event loop; communicate results back to Slint on
  the UI thread using the established worker/channel and timer patterns.
- Avoid accessing Slint widgets from worker threads. Keep UI callbacks thin and
  move state transitions into the existing state/service boundaries where
  appropriate.
- For UI changes, inspect `ui/main.slint`, its Rust bindings, and the relevant
  GUI tests. Preserve keyboard/accessibility behavior, loading and empty states,
  long filename handling, and the supported light/dark/system themes.
- Keep settings versioned and atomically replaced using the existing config
  implementation. Do not write preferences until the user applies/saves them.
- Keep user-visible strings in `i18n/en.toml` and all supported catalogs:
  `pt-BR.toml`, `fr.toml`, `es.toml`, and `it.toml`. Use English as the source
  catalog, preserve matching keys and placeholders, and update localization
  validation/tests and `docs/i18n.md` when the catalog contract changes.
- Preserve command-line compatibility. Update CLI tests and README usage when
  changing flags, defaults, exit behavior, or output.

## Verification

Run checks appropriate to the change. The repository's standard development
commands, from the repository root, are:

```sh
cargo build --workspace --locked
cargo run --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest discover -s scripts -p 'test_*.py'
```

CI also checks formatting, Clippy, workspace tests, the core tests, and a
workspace build. Platform packaging and release behavior are defined in
`.github/workflows/` and documented in `docs/packaging.md`.

- For a focused Rust change, run the relevant package or test target first, then
  the broader checks when practical. Do not weaken or delete a test just to make
  a change pass.
- For Python packaging or workflow changes, run the relevant unittest module
  and inspect the resulting package/workflow behavior. Packaging metadata and
  release scripts can affect distributed binaries; preserve license notices
  and checksums.
- For document-processing changes, add or update synthetic regression tests for
  the changed input or invariant. Where visual output can change, use the
  documented rendered-document verification procedure in addition to
  structural tests.
- For interface or localization changes, run the GUI/catalog tests and follow
  the applicable instructions in `docs/gui-verification.md`, `docs/i18n.md`,
  or `docs/linux-interface.md`. A passing headless test does not prove native
  dialogs, packaging, accessibility, or behavior on another operating system.
- If a check cannot run because a tool, target, display server, or platform is
  unavailable, report that limitation instead of inferring success.

## Documentation and releases

- Update the relevant documentation when changing user-visible behavior,
  supported inputs, settings, CLI, dependencies, packaging, or verification
  requirements. Keep claims tied to evidence and distinguish configured
  workflow steps from checks actually performed.
- Before changing packaging, licensing, dependency notices, or release
  automation, read `docs/dependencies.md`, `docs/packaging.md`, and the relevant
  workflow/script. Do not claim license compliance from a file inventory alone.
- Do not publish releases, push tags, replace remote release assets, or change
  repository secrets/permissions as part of ordinary implementation work.
  Prepare and review the change locally; a release requires an explicit request
  from the repository owner.
- Do not commit build outputs, private fixtures, local configuration, secrets,
  or machine-specific paths. `Cargo.lock` is intentionally tracked and should
  remain consistent with dependency changes.

## Useful references

- `README.md`: current user-facing behavior and development commands.
- `docs/architecture.md`: component boundaries and data flow.
- `docs/proof-of-concept.md`, `docs/image-optimization.md`: engine behavior and
  known limitations.
- `docs/folder-input.md`, `docs/settings.md`, `docs/i18n.md`: application
  behavior and localization contracts.
- `docs/gui-verification.md`, `docs/linux-interface.md`: interface checks and
  platform caveats.
- `docs/dependencies.md`, `docs/packaging.md`, `docs/clean-install.md`,
  `docs/first-release.md`: dependency, distribution, and release requirements.
