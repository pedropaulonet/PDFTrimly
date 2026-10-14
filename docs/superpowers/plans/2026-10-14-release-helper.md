# Release Helper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a guarded Bash menu for staged commits, branch pushes, and tagged releases, and prepare the project for version 0.1.2.

**Architecture:** `scripts/release.sh` handles local preflight and Git operations, requiring explicitly staged changes and confirmation before remote writes. A tagged release pushes `v<version>` and relies on the existing GitHub Actions workflow for package builds and GitHub-generated release notes.

**Tech Stack:** Bash, Git, Cargo, GitHub CLI, existing GitHub Actions workflow.

**Spec:** [results-german-release-design.md](../specs/2026-10-14-results-german-release-design.md)

## Global Constraints

- Never commit arbitrary unstaged or untracked files; do not rewrite or discard user changes.
- Never force-push, overwrite tags, amend commits, or publish releases through a second API path.
- Require explicit confirmation before each remote mutation; explain that a pushed tag can publish a 0.x prerelease after CI succeeds.
- Preserve existing workflow packaging, checksums, and GitHub-generated notes.
- Do not publish a tag or release while implementing this plan.

## Review Focus

- Unstaged or untracked working-tree changes: refuse before commit/tag/push while allowing explicitly staged files; pin in helper tests using a temporary repository.
- Empty staged index: refuse with a useful message; pin in helper tests.
- Existing local or remote version tag: refuse without overwrite; pin in helper tests with a fake remote/`gh`.
- Failed preflight command: stop before commit or push; pin in helper tests with stub commands.
- Normal push versus release selection: only the release choice creates `v<version>`; pin both flows in helper tests.

---

### Task 1: Implement the guarded Bash menu

**Files:**
- Create: `scripts/release.sh`
- Create: `scripts/test_release_helper.py`

**Interfaces:**
- Consumes: staged Git changes, current branch/remote, exact requested version, Cargo, and GitHub CLI for the release path.
- Produces: menu choices `Commit locally`, `Commit and push branch`, `Prepare and push release tag`, and `Quit`; release tags follow `v<version>`.

- [ ] Write temporary-repository tests for the four menu paths, unstaged/untracked refusal, empty index, existing tag, wrong branch, missing `gh` authentication, rejected confirmation, no-previous-tag notes, and command failure; fake `gh` and `git` remotes so no network write occurs.
- [ ] Implement `set -Eeuo pipefail`, repository-root discovery, checks that allow staged changes but reject unstaged/untracked files, staged-diff display, and menu input validation.
- [ ] Implement local commit using only staged files and a required nonempty commit message; never call `git add -A`.
- [ ] Implement branch push with a preview of branch and commit, confirmation, and a normal non-force `git push`.
- [ ] Implement release selection only on `main`, with validated explicit semver, Cargo version/lock update, existing local/remote tag checks, commit confirmation, annotated `v<version>` tag, and a final confirmation before pushing `main` and the tag.
- [ ] Before tag push, show the target tag, commits since the previous tag (or all commits when no earlier tag exists), and that the existing workflow will generate GitHub release notes and can publish a prerelease after Linux and Windows package jobs pass.
- [ ] Run `python3 -m unittest discover -s scripts -p 'test_release_helper.py'` using only temporary repositories and stub executables.

### Task 2: Set and document the 0.1.2 workspace version

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `README.md`
- Modify: `docs/architecture.md`
- Modify: `docs/packaging.md`
- Create: `docs/release-helper.md`

**Interfaces:**
- Consumes: release helper from Task 1 and version target 0.1.2.
- Produces: matching workspace/package version 0.1.2 and maintainer documentation for the menu.

- [ ] Confirm current Cargo workspace version and local lockfile state before changing version metadata.
- [ ] Bump only the workspace package version to 0.1.2 and refresh the corresponding local package entry in `Cargo.lock` without updating dependency versions.
- [ ] Update current user-facing version references in README, architecture, and packaging docs; retain dated 0.1.1 verification and clean-install records as historical evidence.
- [ ] Document required commands, menu choices, staged-only behavior, GitHub CLI authentication, confirmations, automatic release notes, and the effects of pushing a tag.
- [ ] Inspect the final diff to confirm there is no tag, push, release publication, or unrelated Cargo dependency change.
