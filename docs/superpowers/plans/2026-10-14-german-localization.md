# German Localization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add German as a complete embedded language for the desktop app and CLI.

**Architecture:** Append German after Italian in the existing `Language` index, embed `i18n/de.toml`, and extend the current locale, preference, and selector mappings. Preserve English fallback and the current preferences schema.

**Tech Stack:** Rust 2021, embedded TOML catalogs, Slint.

**Spec:** [results-german-release-design.md](../specs/2026-10-14-results-german-release-design.md)

## Global Constraints

- German catalog keys and placeholders must match English exactly.
- Append German after Italian; preserve existing language indexes.
- Preserve locale fallback, CLI behavior, and the preferences schema version.
- Translate the Maximum label and warning introduced by the Maximum compression plan.
- Do not add dependencies; native-speaker review remains required before public release.

## Review Focus

- Locale variants such as `de_DE.UTF-8` and `de-AT`: resolve to German; pin in `src/i18n.rs` tests.
- Unknown locale and `C`: retain English fallback; pin in `src/i18n.rs` tests.
- German preference round trip and old configurations without a language field: pin in `src/config.rs` tests.
- Catalog placeholders and key parity: validate all six catalogs, including Maximum/result strings from the other plans.
- Language changes after batch completion and German filename suffix: pin in `src/app_tests.rs`.

---

### Task 1: Add German catalog and runtime language mapping

**Files:**
- Create: `i18n/de.toml`
- Modify: `src/i18n.rs`
- Modify: `ui/main.slint`
- Modify: `src/app.rs`
- Test: `src/i18n.rs`

**Interfaces:**
- Consumes: current English catalog plus the comparison and Maximum keys from the earlier plans.
- Produces: `Language::German` at index 5, locale base `de`, and embedded sixth catalog.

- [ ] Create `i18n/de.toml` with every English key and matching placeholder; include GUI, CLI, comparison, and Maximum warning strings.
- [ ] Append `German` after `Italian`, extend `SOURCES`/catalog storage from five to six entries, and map `de` and index 5.
- [ ] Add Deutsch after Italiano in the settings selector without changing prior indexes.
- [ ] Add locale and catalog-parity coverage for German and preserve unknown-locale fallback.
- [ ] Run the focused i18n tests and inspect the complete German catalog for untranslated application-owned values.

### Task 2: Persist German and complete user-facing coverage

**Files:**
- Modify: `src/config.rs`
- Modify: `tests/cli.rs`
- Modify: `src/app_tests.rs`
- Modify: `docs/i18n.md`
- Modify: `docs/folder-input.md`
- Test: `src/config.rs`

**Interfaces:**
- Consumes: `Language::German` from Task 1.
- Produces: persisted language string `de`, German CLI/UI text, and German output suffix `komprimiert`.

- [ ] Add `de` preference parse/encode support and include German in all-language preference round trips without changing schema version 1.
- [ ] Extend CLI language coverage from five to six languages and assert German locale/environment selection.
- [ ] Extend GUI coverage for German settings, a completed batch changing language, and the output suffix `komprimiert`.
- [ ] Document German locale detection, persistence, and catalog maintenance; state that native-speaker review is required.
- [ ] Run `cargo test --workspace --locked` and `cargo fmt --all -- --check`.
