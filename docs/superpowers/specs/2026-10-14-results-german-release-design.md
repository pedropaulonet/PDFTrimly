# Results comparison, German catalog, and release helper

## Status and scope

This design covers a proposed PDFTrimly 0.1.2 delivery. The version is a
working target and must be checked against the repository state before release.
The work adds clearer measured batch results, German localization, and an
interactive Bash helper for preparing commits and optionally triggering the
existing GitHub release workflow. It does not add predicted savings or an
in-app AppImage updater.

The repository already measures input/output sizes per PDF, shows a per-file
percentage in the list, shows exact sizes in the selected-file details, and
aggregates successful files in the batch summary. The intent is to surface the
existing measured comparison directly in each completed row rather than add a
second calculation or estimate.

## User-visible results

For a completed PDF, the list row will show the measured original size and
output size, the amount saved, and the savings percentage. The row must remain
readable at the existing minimum window width and with long filenames. The
selected-file detail retains the full output path and exact input/output
result. Failed and not-started rows retain their existing statuses and must
not be presented as successful comparisons. The batch summary continues to
report totals calculated from successful files only.

No estimate is shown before processing. All displayed figures come from the
optimizer's `OptimizeReport`. Existing behavior for files with no reduction,
batch stop, failures, and changing the UI language during a batch is retained.

## German localization

Add German (`de`) as a first-class language throughout the application:

- Embed a German catalog with the same keys and placeholders as English.
- Add German to locale normalization and system detection, the GUI settings
  selector, session language switching, persisted preferences, and CLI text.
- Preserve the existing catalog index ordering by appending German after
  Italian.
- Add a German output-name suffix and update the localization guide and
  automated catalog/locale/config/CLI coverage.

The catalog is a complete draft and should receive native-speaker review
before public release. This review is a release-quality caveat, not a reason to
ship mismatched catalog keys or untranslated application-generated strings.

## Release helper

Add a Bash menu under `scripts/` that guides the maintainer through a normal
commit/push or a tagged release. It uses the tools already required by the
repository (`git`, Cargo, and GitHub CLI for the release path), with no new
runtime dependency. The release path updates the workspace version and lock
file to the chosen version, validates that the tag is unused and matches Cargo
metadata, summarizes the staged changes, and requires explicit confirmation
before committing and pushing. The menu accepts an explicit `x.y.z` version
(with an optional prerelease suffix supported by the existing workflow) and
shows it before applying the bump. A normal push does not create a version tag.
The release choice creates and pushes `v<version>` only after the preflight
and commit steps succeed.

The helper must never commit arbitrary unstaged or untracked files. It checks
the working tree, shows the exact staged diff and intended refs, refuses unsafe
or ambiguous states, and asks before each remote mutation. It must not rewrite
or discard user changes. An absent `gh` login, missing remote, existing tag,
branch mismatch, or failed validation stops the release path with a clear
message.

The current `packages.yml` workflow already builds Linux and Windows packages
on a `v*` tag, then creates/uploads and publishes a prerelease for 0.x after
both package jobs succeed. It also requests GitHub-generated release notes
with `--generate-notes`. The helper will rely on this mechanism and display
the target version/tag and included commits for confirmation; it will not
publish assets itself or create a second release-note source.

## Architecture and boundaries

- Keep size and savings calculations in the application presentation layer,
  using the engine report as the source of truth. Do not add GUI dependencies
  to `pdftrimly-core`.
- Keep German catalog lookup behind `Language` and the existing embedded TOML
  catalog validation. Add German to configuration serialization without
  changing the preferences schema version.
- Keep release orchestration in a standalone maintainer script. It may invoke
  GitHub CLI, but PDF processing remains local and the desktop app gains no
  network behavior.
- Preserve the existing CI/release workflow as the authority for build,
  package checksums, and publication. The helper only prepares and pushes the
  Git state that triggers it.

## Failure handling and safety

Result rows must make it clear when there is no reduction and must not count
failed or cancelled files in savings totals. German fallback behavior remains
English for unsupported locales. Any catalog-key or placeholder mismatch
continues to fail the existing validation.

The release helper stops on the first failed command, never pushes a tag after
a failed preflight/commit, and prints the next manual recovery action. It must
not use force-push, overwrite tags, amend commits, or publish a release through
a second API path. A tag push can lead to a public GitHub prerelease after CI
succeeds; the menu must state this before confirmation.

## Version and documentation

Use 0.1.2 as the target only after checking the current version in Cargo and
the lockfile. Update user-facing current-version statements and relevant
localization/release-helper documentation. Keep historical 0.1.1 verification
records intact. Do not publish a tag or release as part of implementing this
design.

## Verification plan

- Check per-file row rendering for successful, unchanged, failed, and stopped
  entries at narrow and normal window sizes; verify detail paths and aggregate
  totals still use measured successful results.
- Validate every catalog key and placeholder, German locale normalization,
  persisted preference round trips, CLI help/errors/results, language changes
  after a batch, and the German output suffix.
- Exercise the Bash helper with a temporary Git repository and stubbed GitHub
  CLI so normal commit/push, release confirmation, preflight rejection, and
  command failures can be checked without changing a real remote.
- Run the repository's applicable formatting, build, lint, test, and packaging
  checks before claiming the implementation complete. Native German language
  review and actual GitHub publication remain separate release checks.
