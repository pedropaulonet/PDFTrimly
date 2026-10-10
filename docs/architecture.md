# Foundation architecture

The project follows `projeto.md`: a Rust workspace containing
the `pdftrimly` application and an independent `pdftrimly-core` library.
Both inherit version 0.1.0, edition 2021, authorship and MIT metadata from
the workspace. Unsafe Rust is forbidden in the engine and denied in handwritten application
code. Slint-generated vtable macros carry their own local allowances.

The application opens Slint without arguments and provides a `compress` command, help and version.
`app.rs` connects Slint callbacks, native dialogs and worker channels.
Window construction is separate from running its event loop; a `FilePicker`
boundary allows `app_tests.rs` to exercise the production callbacks, timers and
worker using an in-memory Slint platform and supplied picker responses.
`gui_state.rs` owns semantic selection, results and batch state; results are
rendered again when the session language changes. `i18n.rs` loads embedded TOML
catalogs, normalizes locale and validates translation keys and placeholders.
`batch.rs` runs sequential jobs and reserves exclusive output names. The core
implements lossless stream optimization in `streams.rs`, file orchestration
in `optimizer.rs` and structural checks in `validation.rs`. `image_usage.rs`
tracks image placement through pages and forms; `images.rs` resizes and
recompresses eligible RGB/gray images for Normal and Ultra. Light leaves
image data intact. Existing OCR text and page drawing commands are preserved.

## Dependency direction

`pdftrimly` → `pdftrimly-core`

The core does not depend on the GUI. `optimize(input, output, options)`
returns an `OptimizeReport` with measured sizes, outcome, page count,
optimized/skipped stream counts and duration. File errors and unsupported
documents produce an `OptimizeError`. See `docs/proof-of-concept.md`.

Preferences are loaded before constructing the window. `config.rs` handles
versioned TOML and atomic replacement; `settings.rs` handles drafts and applying
saved defaults. `actions.rs` and `desktop.rs` separate UI callbacks from native
clipboard/launcher services. `ui_tasks.rs` publishes asynchronous I/O results
on the UI thread using timers; the engine knows nothing about these services.

## Next stages

See `docs/first-release.md` for the current requirements audit. Priorities are
complete graphical verification and real-world document coverage,
cooperative cancellation, and release packaging.
The core stays independent of translation catalogs and UI callbacks. Batch
errors cross the worker channel as typed errors and are translated by the
application at presentation time.

`discovery.rs` discovers canonical PDF inputs and records their source roots.
`inputs.rs` adapts picker/drop callbacks to asynchronous discovery. The batch
receives explicit relative directories; presentation stays in `gui_state.rs`
and Slint. `batch.rs` checks destination containment and reserves output names.
