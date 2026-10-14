# Maintainer release helper

Run `./scripts/release.sh` from a clean checkout. Stage only the files intended
for the operation before opening the menu. The helper refuses unstaged and
untracked files, displays the staged summary and never runs `git add -A`.

Choices are: create a local commit, create a commit and push the current branch,
or prepare a versioned release tag. Commit choices require a nonempty message.
The branch push is a normal, non-force push and asks for confirmation.

The release choice is restricted to `main`. It requires Cargo, GitHub CLI, an
authenticated `gh` session and a remote. Enter an explicit `x.y.z` version (an
optional prerelease suffix is accepted). The helper refuses a tag already
present locally or remotely, updates the workspace version and lockfile, then
creates a release commit and annotated `v<version>` tag. It asks again before
pushing `main` and the tag. A pushed `v*` tag triggers the existing GitHub
Actions package workflow; for 0.x it can publish a prerelease after the Linux
and Windows jobs succeed. That workflow generates the GitHub release notes.

The helper does not publish a release through a separate API call. Review the
generated workflow run and release on GitHub after pushing a tag. German copy
also needs native-speaker review before public release.
