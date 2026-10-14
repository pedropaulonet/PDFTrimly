# Batch Results Comparison Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show measured input/output sizes and actual savings for each completed PDF in the batch list.

**Architecture:** Format the existing `OptimizeReport` in the application layer and expose that string through a `PdfRow` property. Keep the batch total and selected-file detail as existing sources of truth; do not change `pdftrimly-core` or estimate savings.

**Tech Stack:** Rust 2021, Slint, embedded TOML localization.

**Spec:** [results-german-release-design.md](../specs/2026-10-14-results-german-release-design.md)

## Global Constraints

- Use measured values from `OptimizeReport`; show no estimate before processing.
- Count only successful files in aggregate input/output totals.
- Keep `pdftrimly-core` independent from GUI and localization.
- Preserve original PDFs and existing temporary-output, validation, and no-overwrite safeguards.
- Do not add dependencies or commit private PDF fixtures.

## Review Focus

- Unchanged output: show equal measured sizes and the localized no-savings status; pin in `src/gui_state.rs` coverage.
- Failed or cancelled rows: do not present them as successful comparisons; pin in the GUI integration test.
- Long filenames at the 680 px minimum width: keep comparison values visible; pin in the existing large-list/UI coverage.
- Language changes after completion: rerender the comparison in the new locale; pin in the batch language-change integration test.
- Mixed success and failure totals: aggregate only successful PDF sizes; pin in the batch summary state test.

---

### Task 1: Format exact per-file comparisons

**Files:**
- Modify: `i18n/en.toml`
- Modify: `i18n/pt-BR.toml`
- Modify: `i18n/fr.toml`
- Modify: `i18n/es.toml`
- Modify: `i18n/it.toml`
- Modify: `src/messages.rs`
- Test: `src/gui_state.rs`

**Interfaces:**
- Consumes: `pdftrimly_core::OptimizeReport` and `Language::text/format/decimal`.
- Produces: `messages::comparison(language: Language, report: &OptimizeReport) -> String`, used by the UI row model.

- [ ] Add state coverage for a reduced report and an unchanged report, asserting original/final sizes and the correct no-savings status; run the focused test and confirm it fails for the missing comparison.
- [ ] Add a localized `comparison` message with `{input}`, `{output}`, `{saved}`, and `{savings}` placeholders and a localized no-savings label in the five current catalogs.
- [ ] Add `messages::comparison`; format sizes using `messages::size`, saved bytes using `input_bytes.saturating_sub(output_bytes)`, and percentage using `report.savings_percent()`; for `Outcome::Unchanged`, show the no-savings label.
- [ ] Run `cargo test --workspace --locked` and confirm the comparison state coverage passes.

### Task 2: Display the comparison in each GUI row

**Files:**
- Modify: `ui/main.slint`
- Modify: `src/app.rs`
- Modify: `src/gui_state.rs`
- Modify: `src/app_tests.rs`
- Modify: `docs/folder-input.md`

**Interfaces:**
- Consumes: `messages::comparison` from Task 1.
- Produces: `PdfRow.comparison: string`, populated with measured comparison text for successful rows and the existing input size for rows without a result.

- [ ] Add GUI integration assertions for successful, unchanged, failed, cancelled, locale-switch, and batch-total row states; run the focused integration test and confirm it fails because the row comparison is missing.
- [ ] Add `comparison: string` to `PdfRow` and populate it in `refresh` from `RowStatus::Finished(Ok((report, _)))`; keep pending, failed, and cancelled rows on their current status path.
- [ ] Render the comparison in the existing list row, adjusting column widths while retaining the minimum window width and long-name elision.
- [ ] Update `docs/folder-input.md` to describe the per-row measured comparison and batch summary.
- [ ] Run `cargo test --workspace --locked` and inspect the existing GUI row rendering at 680 px and normal width.
