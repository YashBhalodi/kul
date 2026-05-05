# Release process

KulaLang ships three things from one repository: the `kula` CLI, the `kula-lsp` language server, and the VSCode marketplace extension. They release in lockstep — one tag, one pipeline, one set of coordinated artifacts.

This doc is the source of truth for how to cut a release and what the pipeline does.

## Overview

Pushing a tag of the form `v<major>.<minor>.<patch>` triggers `.github/workflows/release.yml`, which:

1. **Verifies version coordination** — `Cargo.toml` workspace version, `editor/vscode/package.json` version, and the tag must all match. Fails fast if they don't.
2. **Builds `kula` for four targets** — `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`. Each binary is smoke-tested with `kula --version` and `kula validate examples/03-three-generations.kula`.
3. **Builds `kula-lsp` for the same four targets**, smoke-tested with `kula-lsp --version`.
4. **Creates a GitHub Release** at `v<version>` with all eight archives attached and auto-generated release notes.
5. **Publishes the VSCode extension** to the marketplace, with platform-specific `kula-lsp` binaries bundled inside the `.vsix` so end users don't need to set `kula.serverPath`.

`build-cli` and `build-lsp` run in parallel after `verify`; `github-release` and `marketplace-publish` run in parallel after their respective build jobs finish. Total wall-clock time is dominated by the slowest matrix build.

```
verify ──┬──► build-cli (4 targets) ──┐
         │                             ├──► github-release   (8 archives)
         └──► build-lsp (4 targets) ──┴──► marketplace-publish (bundled .vsix)
```

## Cutting a release

Three files must agree on the version before tagging:

- `Cargo.toml` → `[workspace.package].version`
- `editor/vscode/package.json` → `version`
- The git tag → `v<version>` (no prefix, no suffix beyond patch)

The `verify` job in the release workflow rejects any drift, so you can't accidentally ship a misaligned release.

### Procedure

```sh
# 1. Bump versions to match.
#    For v0.1.0 the workspace and extension are already at 0.1.0 — skip ahead.
$EDITOR Cargo.toml editor/vscode/package.json

# 2. Commit and push the bump.
git commit -am "Bump version to 0.x.0"
git push origin main

# 3. (Optional) Dry-run the pipeline before tagging.
#    GitHub UI → Actions → Release → Run workflow → leave dry_run: true
#    Builds + smoke-tests every binary on every platform without publishing.

# 4. Tag and push.
git tag v0.x.0
git push origin v0.x.0
```

The pipeline runs automatically. Watch the progress at https://github.com/YashBhalodi/kulalang/actions.

### What "done" looks like

- GitHub Release at `https://github.com/YashBhalodi/kulalang/releases/tag/v0.x.0` carries 8 archives:
  - `kula-<target>.{tar.gz,zip}` × 4
  - `kula-lsp-<target>.{tar.gz,zip}` × 4
- Marketplace listing at `https://marketplace.visualstudio.com/items?itemName=YashBhalodi.kulalang` shows the new version.
- `code --install-extension YashBhalodi.kulalang` on a clean profile installs and works without setting `kula.serverPath` (the extension auto-locates the right platform binary from the bundled `server/<platform>/`).

### Recommended post-publish smoke

The integration tests cover protocol correctness, but only a human catches "the squiggle color is wrong on this theme" or "the hover popover is hard to read". After a release lands:

- Open each `examples/*.kula` in real VSCode (clean profile, bundled binary).
- Exercise diagnostics, hover, go-to-definition, completion on both light and dark themes.

## One-time setup

Before the very first marketplace publish, the publisher account and PAT must exist. The historical blocker is Microsoft Account credit-card verification — have a card handy.

### a. Create the marketplace publisher

- https://marketplace.visualstudio.com/manage
- Sign in with a Microsoft account; complete credit-card verification
- Create publisher with ID exactly `YashBhalodi` (case-sensitive, must match `editor/vscode/package.json`'s `"publisher"`)

### b. Generate the publishing PAT

- https://dev.azure.com/ → User Settings → Personal Access Tokens → New Token
- Organization: **All accessible organizations**
- Expiration: 1 year is typical
- Scopes: **Custom defined** → expand **Marketplace** → check **Manage**
- Copy the token immediately (only shown once)

### c. Store as repo secret

```sh
gh secret set VSCE_PAT
# paste the token at the prompt
```

The publish job reads `VSCE_PAT` from secrets. Anytime the PAT expires, repeat (b) and (c) — `vsce publish` will start failing with a 401 until refreshed.

## Pipeline reference

### `verify`

Parses the version from the tag (or skips when triggered by `workflow_dispatch`), reads the workspace and extension versions, fails if any pair disagrees. Outputs the resolved version for downstream jobs.

### `build-cli` / `build-lsp`

Standard cross-compilation matrix. Each platform target builds in release mode with `Swatinem/rust-cache` for incremental speed. Smoke tests run on the platforms where they can — `x86_64-apple-darwin` is skipped because `macos-latest` is now ARM-based and would need Rosetta to execute.

`build-lsp` uploads two artifact sets per platform:

- An archive (`kula-lsp-<target>.{tar.gz,zip}`) for the GitHub Release.
- A raw binary under `kula-lsp-raw-<platform_dir>/` for the marketplace job to bundle directly. This avoids re-downloading from a Release the workflow itself just produced.

### `github-release`

Pulls every archive artifact, copies them into a flat directory, and creates the public Release with `softprops/action-gh-release@v2`. Release notes are auto-generated from PRs/commits since the previous release.

### `marketplace-publish`

Pulls the raw `kula-lsp` binaries, stages them under `editor/vscode/server/<platform>/`, runs `npm ci`, and runs `vsce publish` with `VSCE_PAT`. The bundled `.vsix` carries all four platform binaries; VSCode's marketplace doesn't currently support platform-specific extension splits for this workflow shape, but if size becomes a concern that's a future-friendly migration path.

### `dry_run`

`workflow_dispatch` accepts a `dry_run` input (default `true`). When true, the conditional `if:` on `github-release` and `marketplace-publish` evaluates false, so the pipeline builds + smoke-tests every binary without publishing anything. Tag pushes always set `dry_run` effectively to false because the `if:` short-circuits on `github.event_name == 'push'`.

## Troubleshooting

### `verify` fails

The error message identifies which two of (workspace, extension, tag) disagree. Edit the offending file, re-tag if the tag was the wrong one (`git tag -d v0.x.0 && git push --delete origin v0.x.0` then re-tag).

### A `build-*` job fails

Look at the failing matrix entry. Most failures are a transient toolchain issue or a clippy/test regression — fix the underlying issue on `main`, push a new commit, and re-tag.

### `marketplace-publish` fails with 401

PAT expired or doesn't have **Marketplace → Manage** scope. Refresh per the one-time-setup steps and re-run the failed job.

### Release exists but extension didn't publish

Both `github-release` and `marketplace-publish` are independent — one failing doesn't roll back the other. If the extension publish failed but the Release succeeded, fix the underlying issue and re-run just the `marketplace-publish` job (Actions UI → workflow run → "Re-run failed jobs"). The `vsce publish` step is idempotent for already-published versions; if the marketplace already accepted that version you'll need to bump and re-cut.

### Need to ship a fix

The semver-bump-and-tag procedure is the only path forward. There's no concept of "amending" a published release — bump to the next patch (or revert with a higher version if the broken release is in users' hands).

## Related

- The unified release workflow: [`.github/workflows/release.yml`](../.github/workflows/release.yml)
- Per-PR extension lint: [`.github/workflows/vscode-extension.yml`](../.github/workflows/vscode-extension.yml)
- Local-dev install paths for the extension: [`editor/vscode/README.md`](../editor/vscode/README.md)
