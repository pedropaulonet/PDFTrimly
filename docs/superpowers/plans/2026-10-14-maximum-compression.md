# Maximum Compression Level Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a fourth, clearly warned Maximum compression preset at 72 DPI and JPEG quality 40.

**Architecture:** Add the preset to the engine enum and image-quality mapping, then pass it through the existing GUI, CLI, and preference boundaries. Reuse existing image eligibility, decoding limits, output validation, and unchanged-output handling.

**Tech Stack:** Rust 2021, Slint, TOML catalogs, existing `image` and `lopdf` dependencies.

**Spec:** [results-german-release-design.md](../specs/2026-10-14-results-german-release-design.md)

## Global Constraints

- Maximum uses 72 DPI and JPEG quality 40; Ultra remains 96 DPI and quality 60.
- Normal remains the default for existing users and configurations.
- Preserve unsupported image data, OCR/searchable text, original files, and no-overwrite safeguards.
- Keep the engine independent from GUI and localization.
- Do not add or replace dependencies; do not commit private PDF fixtures.

## Review Focus

- Image placements above 72 DPI: downsample only eligible images to the Maximum target; pin in `crates/pdftrimly-core/tests/images.rs`.
- Image placements already below target: never enlarge images; pin in the same core test module.
- Existing OCR text and page drawing commands: preserve both through Maximum; pin in the OCR regression fixture test.
- Unsupported filters, masks, or incomplete placement analysis: preserve original image streams; pin in the existing preservation tests.
- No material savings: preserve existing unchanged-output behavior and never publish a larger candidate; pin in `crates/pdftrimly-core/tests/optimizer.rs`.

---

### Task 1: Add the Maximum engine preset

**Files:**
- Modify: `crates/pdftrimly-core/src/lib.rs`
- Modify: `crates/pdftrimly-core/src/images.rs`
- Test: `crates/pdftrimly-core/tests/images.rs`
- Test: `crates/pdftrimly-core/tests/optimizer.rs`

**Interfaces:**
- Consumes: `CompressionLevel` passed through `OptimizeOptions`.
- Produces: `CompressionLevel::Maximum`; `images.rs` maps it to `(72.0, 40)`.

- [ ] Add a synthetic image regression for Maximum output dimensions at a known placement size and confirm the decoded OCR layer and page drawing commands remain unchanged; run it and confirm it fails because `Maximum` is missing.
- [ ] Add `Maximum` to `CompressionLevel` and select 72 DPI / quality 40 in the image optimizer.
- [ ] Add or extend an optimizer case to verify unchanged-or-smaller output and source preservation when the candidate does not save material bytes.
- [ ] Extend existing unsupported-image, incomplete-placement, and resource-bound cases to run with Maximum and assert the original image data is preserved or skipped safely.
- [ ] Run `cargo test -p pdftrimly-core --locked` and inspect the relevant image/optimizer results.

### Task 2: Expose Maximum in settings, GUI, and CLI

**Files:**
- Modify: `src/config.rs`
- Modify: `src/main.rs`
- Modify: `src/app.rs`
- Modify: `ui/main.slint`
- Modify: `i18n/en.toml`
- Modify: `i18n/pt-BR.toml`
- Modify: `i18n/fr.toml`
- Modify: `i18n/es.toml`
- Modify: `i18n/it.toml`
- Test: `src/config.rs`
- Test: `src/app_tests.rs`
- Test: `tests/cli.rs`

**Interfaces:**
- Consumes: `CompressionLevel::Maximum` from Task 1.
- Produces: GUI and CLI selection of `maximum`, plus persisted preference value `maximum`; UI copy exposes a localized `maximum` label and `maximum_hint` warning.

- [ ] Add CLI, configuration, and GUI assertions for selecting Maximum, retaining Normal as default, and showing the warning; run the focused cases and confirm they fail because the level is missing.
- [ ] Add `maximum` and `maximum_hint` catalog keys to the five current languages and bind them through `Labels` and `apply_labels`.
- [ ] Append Maximum to the GUI level selectors and add enum-index conversion in `src/app.rs` and persisted string conversion in `src/config.rs`, keeping Normal's default index unchanged.
- [ ] Extend CLI parsing/help to accept `--level maximum`; retain existing values and defaults.
- [ ] Run `cargo test --workspace --locked` and `cargo fmt --all -- --check`, confirming the Maximum settings, GUI, and CLI coverage passes.

### Task 3: Document visual tradeoffs

**Files:**
- Modify: `docs/image-optimization.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: public level name and parameters from Tasks 1–2.
- Produces: user-facing documentation of 72 DPI / quality 40 and significant image quality loss.

- [ ] Update the level table and describe that small text in scans may become difficult to read, while searchable/OCR text is preserved where supported.
- [ ] State that savings vary by document and no minimum savings is guaranteed.
- [ ] Review docs for consistency with the actual CLI and UI copy.
