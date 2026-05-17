# Release process

KulLang ships four things from one repository: the `kul` CLI, the `kul-lsp` language server, the VSCode extension (published to both Open VSX and the VS Code Marketplace), and the `@kullang/wasm` npm package. They release in lockstep — one tag, one pipeline, one set of coordinated artifacts.

This doc is the source of truth for how to cut a release and what the pipeline does.

## Overview

Pushing a tag of the form `v<major>.<minor>.<patch>` triggers `.github/workflows/release.yml`, which:

1. **Verifies version coordination** — `Cargo.toml` workspace version, `editor/vscode/package.json` version, and the tag must all match. Fails fast if they don't. The `wasm-publish` job re-asserts the wasm-pack-produced npm `package.json` version against the same tag as a belt-and-braces guard.
2. **Builds `kul` for four targets** — `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`. Each binary is smoke-tested with `kul --version` and `(cd examples/03-three-generations && kul validate)` (the CLI is CWD-rooted per issue #83 — every subcommand discovers the project from the current directory).
3. **Builds `kul-lsp` for the same four targets**, smoke-tested with `kul-lsp --version`.
4. **Builds and publishes `@kullang/wasm`** — `wasm-pack build --target bundler`, gzipped bundle-size assertion, `npm publish --access public`, and a `kul-wasm.tar.gz` artifact for the GitHub Release.
5. **Publishes the VSCode extension** to [Open VSX](https://open-vsx.org/) and the [VS Code Marketplace](https://marketplace.visualstudio.com/) as four platform-specific `.vsix` files (one each for `darwin-arm64`, `darwin-x64`, `linux-x64`, `win32-x64`), each carrying the matching `kul-lsp` binary so end users don't need to set `kul.serverPath`. Each `.vsix` is also uploaded as a workflow artifact (`kul-vsix-<target>`) so the GitHub Release can attach it for direct-download users.
6. **Creates a GitHub Release** at `v<version>` with all 13 artifacts attached (8 CLI/LSP archives + 1 WASM tarball + 4 platform-specific `.vsix`) and auto-generated release notes.

`build-cli`, `build-lsp`, and `wasm-publish` run in parallel after `verify`; `extension-publish` runs as a 4-target matrix after `build-lsp` (each matrix entry bundles a single platform's LSP binary into a `--target`-tagged `.vsix` and pushes it to both registries); `github-release` waits for all four (it consumes their artifacts). Total wall-clock time is dominated by the slowest matrix build.

```
verify ──┬──► build-cli      (4 targets) ─────────────────────────┐
         │                                                         │
         ├──► build-lsp      (4 targets) ──► extension-publish ────┼──► github-release   (13 artifacts)
         │                                       (4× matrix)       │
         │                                              │          │
         │                                              ├──► Open VSX           (per-platform .vsix × 4)
         │                                              └──► VS Code Marketplace (per-platform .vsix × 4)
         └──► wasm-publish ──► npm (@kullang/wasm) ──┘
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

The pipeline runs automatically. Watch the progress at https://github.com/YashBhalodi/kul/actions.

### What "done" looks like

- GitHub Release at `https://github.com/YashBhalodi/kul/releases/tag/v0.x.0` carries 13 artifacts:
  - `kul-<target>.{tar.gz,zip}` × 4
  - `kul-lsp-<target>.{tar.gz,zip}` × 4
  - `kul-wasm.tar.gz` × 1
  - `kul-0.x.0-<target>.vsix` × 4 (`darwin-arm64`, `darwin-x64`, `linux-x64`, `win32-x64`)
- Open VSX listing at `https://open-vsx.org/extension/YashBhalodi/kul` shows the new version with a platform dropdown exposing all four downloads.
- VS Code Marketplace listing at `https://marketplace.visualstudio.com/items?itemName=YashBhalodi.kul` shows the new version. The Marketplace silently serves the matching platform `.vsix` to each client; there's no UI dropdown.
- On an Open-VSX-consuming editor (VSCodium, Cursor, Windsurf, Theia/Che, Gitpod), `<editor> --install-extension YashBhalodi.kul` resolves against Open VSX, picks the matching platform `.vsix`, installs, and works without setting `kul.serverPath` (the extension auto-locates the bundled binary from `server/<platform>/`).
- On upstream VSCode, `code --install-extension YashBhalodi.kul` resolves against the Marketplace and installs the matching platform `.vsix`. Same bundled-binary autolocation behavior. The released `.vsix` files remain attached to the GitHub Release for users who prefer to sideload (`code --install-extension /path/to/kul-0.x.0-<target>.vsix`).
- `npm view @kullang/wasm version` returns `0.x.0`. A clean Node project can `npm install @kullang/wasm@0.x.0` and `import { check, exportGraph, format } from '@kullang/wasm'` without further setup.

### Recommended post-publish smoke

The integration tests cover protocol correctness, but only a human catches "the squiggle color is wrong on this theme" or "the hover popover is hard to read". After a release lands:

- Open each `examples/*/*.kul` in real VSCode (clean profile, bundled binary).
- Exercise diagnostics, hover, go-to-definition, completion on both light and dark themes.

## One-time setup

The extension publishes to two registries with independent setup paths. Both must be completed before the first tagged release; thereafter the only maintenance work is rotating the two PATs when they expire.

### Open VSX

Open VSX is free — no payment, no KYC, just an Eclipse account and a namespace claim.

#### a. Create the Eclipse account

- https://accounts.eclipse.org → Register (or sign in)
- Email verification only; no payment, no KYC

#### b. Sign in to Open VSX and accept the Publisher Agreement

- https://open-vsx.org → "Log in" → GitHub OAuth
- Profile menu → user-settings → click "Show Publisher Agreement" and accept the terms
- The GitHub identity used for OAuth must be linked to (or share an email with) the Eclipse account

#### c. Generate the publishing PAT

- https://open-vsx.org/user-settings/tokens → "Generate New Token"
- Description: anything memorable (e.g. `kul-release-ci`)
- Copy the token immediately (only shown once)

#### d. Pre-claim the `YashBhalodi` namespace

The namespace name is case-sensitive — it must match `editor/vscode/package.json`'s `"publisher"` field. Open VSX does not auto-create namespaces on first publish, so this step must happen before tagging:

```sh
npx --yes ovsx create-namespace YashBhalodi --pat <token>
```

The namespace is also subject to a one-time ownership verification by an Open VSX maintainer (file an issue with the "Claim namespace ownership" template at https://github.com/EclipseFdn/open-vsx.org/issues/new/choose). Until verified, the published listing carries an "unverified namespace" warning, but publishing works. Tracking issue for KulLang: https://github.com/EclipseFdn/open-vsx.org/issues/10180.

#### e. Store as repo secret

```sh
gh secret set OVSX_PAT
# paste the token at the prompt
```

The publish job reads `OVSX_PAT` from secrets. Anytime the PAT expires or is rotated, repeat (c) and (e) — `ovsx publish` will start failing with a 401 until refreshed.

### VS Code Marketplace

The Marketplace is gated on an Azure DevOps account. Extension publishing itself is free, but creating the Azure DevOps organization that backs the publisher requires linking a credit card (free trial is sufficient — no charges if you stay within the free tier).

#### a. Create the Azure DevOps account and publisher

- https://dev.azure.com → sign in with a Microsoft account → create an organization (any name, region nearest you)
- https://marketplace.visualstudio.com/manage/createpublisher → create a publisher with `ID = YashBhalodi` (case-sensitive, must match `editor/vscode/package.json`'s `"publisher"` field)
- Confirm at https://marketplace.visualstudio.com/manage/publishers/YashBhalodi — you should land on the publisher dashboard. If you don't, the Microsoft account isn't an owner of the publisher.

#### b. Generate the publishing PAT

- https://dev.azure.com → User settings (top right) → Personal Access Tokens → New Token
- Organization: **All accessible organizations** (required — Marketplace is global, not org-scoped)
- Scopes: Custom defined → expand **Marketplace** → check **Manage**
- Expiration: 1 year is reasonable
- Copy the token immediately (only shown once)

#### c. Store as repo secret

```sh
gh secret set VSCE_PAT
# paste the token at the prompt
```

The publish job reads `VSCE_PAT` from secrets. Anytime the PAT expires or is rotated, repeat (b) and (c) — `vsce publish` will start failing with a 401 until refreshed.

## Pipeline reference

### `verify`

Parses the version from the tag (or skips when triggered by `workflow_dispatch`), reads the workspace and extension versions, fails if any pair disagrees. Outputs the resolved version for downstream jobs.

### `build-cli` / `build-lsp`

Standard cross-compilation matrix. Each platform target builds in release mode with `Swatinem/rust-cache` for incremental speed. Smoke tests run on the platforms where they can — `x86_64-apple-darwin` is skipped because `macos-latest` is now ARM-based and would need Rosetta to execute.

`build-lsp` uploads two artifact sets per platform:

- An archive (`kul-lsp-<target>.{tar.gz,zip}`) for the GitHub Release.
- A raw binary under `kul-lsp-raw-<platform_dir>/` for the `extension-publish` job to bundle directly. This avoids re-downloading from a Release the workflow itself just produced.

### `wasm-publish`

Builds the `@kullang/wasm` package via `wasm-pack build --target bundler`, rewrites the wasm-pack-generated `package.json` `name` to `@kullang/wasm` (wasm-pack derives the npm name from the Rust crate name `kul-wasm`), asserts the gzipped `.wasm` is ≤ 1 MB, and re-asserts the npm `package.json` version equals the release version. The `pkg/` output is then staged into `kul-wasm/` and packaged as `kul-wasm.tar.gz`, uploaded as the `kul-wasm` artifact for `github-release` to attach to the public Release.

On a real publish (tag push or `dry_run: false`), the job also runs `npm publish --access public` from `crates/kul-wasm/pkg`, authenticated via the `NPM_TOKEN` repo secret. A pre-flight step fails with a readable error if `NPM_TOKEN` is unset, matching the `OVSX_PAT` failure shape. On dry-run, the build and the version assertions still run — only the npm publish is skipped, so dry-runs catch breakage before tagging.

The job does not re-run `cargo test`, the Node smoke, or `tsc --noEmit` — `.github/workflows/rust.yml`'s `wasm-build` job already gates the merge to `main`, so any commit a tag points at has already passed those checks.

### `github-release`

Pulls every archive artifact, copies them into a flat directory, and creates the public Release with `softprops/action-gh-release@v2`. Release notes are auto-generated from PRs/commits since the previous release.

### `extension-publish`

Runs as a 4-target matrix (`darwin-arm64`, `darwin-x64`, `linux-x64`, `win32-x64`). Each matrix entry pulls only its raw `kul-lsp` binary, stages it under `editor/vscode/server/<target>/`, `chmod +x`s it (belt-and-suspenders for vsce's zip layer), runs `npm ci`, and packages a platform-tagged extension via `vsce package --target <target>` (no global install — invoked through `npx @vscode/vsce` from the extension's `devDependencies`-resolved transitive). Each `.vsix` is uploaded as a workflow artifact named `kul-vsix-<target>` so `github-release` can attach it to the public Release.

On a real publish (tag push or `dry_run: false`), each matrix entry runs two publishes in sequence:

- `npx ovsx publish kul-<version>-<target>.vsix -p $OVSX_PAT` to Open VSX.
- `npx @vscode/vsce publish --packagePath kul-<version>-<target>.vsix --pat $VSCE_PAT` to the VS Code Marketplace.

Both pre-flight checks (for `OVSX_PAT` and `VSCE_PAT`) run before either publish, so a missing secret fails fast without leaving the two registries out of sync. The Open VSX publish runs first; if it succeeds and the Marketplace publish fails, you cannot retry the matrix entry as-is (Open VSX rejects republishing the same version+target). Bumping the version and re-tagging is the supported recovery — see Troubleshooting below.

On dry-run, the package step and the artifact upload still run — only the two publish steps are skipped, so dry-runs catch breakage before tagging.

Per-platform packaging is required: an "untagged" `.vsix` (no `--target`) is treated as platform-independent by Cursor's marketplace install path, which strips the `server/` directory on the assumption that platform-specific binaries are stale. The `--target` flag stamps `targetPlatform` into the manifest so each registry serves the matching `.vsix` to each user and preserves its bundled binary intact.

### `dry_run`

`workflow_dispatch` accepts a `dry_run` input (default `true`). When true, the conditional `if:` on `github-release` and on the `extension-publish` publish steps evaluates false, so the pipeline builds + smoke-tests every binary, packages the four `.vsix` files, and uploads the artifacts without publishing anything. Tag pushes always set `dry_run` effectively to false because the `if:` short-circuits on `github.event_name == 'push'`.

## Troubleshooting

### `verify` fails

The error message identifies which two of (workspace, extension, tag) disagree. Edit the offending file, re-tag if the tag was the wrong one (`git tag -d v0.x.0 && git push --delete origin v0.x.0` then re-tag).

### A `build-*` job fails

Look at the failing matrix entry. Most failures are a transient toolchain issue or a clippy/test regression — fix the underlying issue on `main`, push a new commit, and re-tag.

### `extension-publish` fails (Open VSX step)

- **`OVSX_PAT secret is unset`** — the pre-flight check ran. Set the secret per the one-time setup steps and re-run the failed job.
- **`401 Unauthorized` from `ovsx publish`** — the token expired, was revoked, or was generated against a different Eclipse account. Generate a fresh token at https://open-vsx.org/user-settings/tokens and `gh secret set OVSX_PAT`.
- **`Namespace 'YashBhalodi' does not exist`** — the namespace pre-claim was never run, or it was claimed under a different account. Run `npx --yes ovsx create-namespace YashBhalodi --pat <token>` once with the same token now stored in `OVSX_PAT`.
- **Secret-scanner rejection on upload** — Open VSX runs an automated scan and refuses uploads it flags. The error message identifies the offending file inside the `.vsix`. Fix at the source (typically a stray `.env`, key, or test fixture that landed in the bundled extension), bump and re-tag.
- **`Extension YashBhalodi.kul <version> already exists`** — `ovsx publish` is not idempotent on a published version. Bump the workspace version (and `editor/vscode/package.json`), re-tag, and re-run the pipeline.

### `extension-publish` fails (VS Code Marketplace step)

- **`VSCE_PAT secret is unset`** — the pre-flight check ran. Set the secret per the one-time setup steps and re-run the failed job.
- **`401 Unauthorized` from `vsce publish`** — the token expired, was revoked, was generated against the wrong Azure DevOps account, or was scoped to a single organization instead of "All accessible organizations". Regenerate the PAT with **Marketplace > Manage** scope and **All accessible organizations**, then `gh secret set VSCE_PAT`.
- **`The Personal Access Token verification has failed`** — same root cause as above; the PAT is missing the Marketplace scope. Regenerate with the correct scope.
- **`The publisher 'YashBhalodi' is unknown`** — the publisher was never created (or was created under a different Microsoft account). Visit https://marketplace.visualstudio.com/manage/createpublisher and create it with `ID = YashBhalodi`.
- **`A extension with this name already exists in the Marketplace`** at first publish — name collision with another publisher's extension. Marketplace enforces a global name uniqueness across publishers for the `name` field; this only matters on the very first push. If you hit it, rename `editor/vscode/package.json`'s `"name"` (the displayed slug, not the publisher) to something unique.
- **`<version> already exists`** — `vsce publish` is not idempotent on a published version. Bump and re-tag, same as the Open VSX recovery.

### `wasm-publish` fails on the npm publish step

- **`NPM_TOKEN secret is unset`** — the pre-flight check ran. Set the secret per the one-time npm setup steps and re-run the failed job.
- **`E401`/`E403` from `npm publish`** — the token expired, was revoked, or lacks publish permission on the `@kullang` scope. Generate a fresh automation token at npmjs.com (Account → Access Tokens → Automation) and `gh secret set NPM_TOKEN`.
- **`E403 — cannot publish over existing version`** — the version was already published. `npm publish` is not idempotent; bump the workspace version (and `editor/vscode/package.json`), re-tag, and re-run the pipeline.
- **`E402 — payment required` on first publish** — the `@kullang` scope is private-by-default. The job already passes `--access public`; if this surfaces, the scope-claim setup wasn't completed (see issue #36's one-time npm setup section).

### Release exists but extension didn't publish

`github-release` depends on `extension-publish`, so a publish failure on any of the four matrix entries (to either registry) blocks the Release from being created in the first place. If the failure was transient (network blip, registry hiccup), fix any underlying issue and re-run the failed jobs (Actions UI → workflow run → "Re-run failed jobs"); the per-platform `kul-vsix-<target>` artifacts uploaded earlier in the same job are reused.

If a registry already accepted that version-and-target but the re-run mistakes it for a fresh attempt, you'll need to bump and re-cut — neither `ovsx publish` nor `vsce publish` is idempotent on the same version+target. The only remaining recovery is a version bump.

The Open VSX publish runs before the Marketplace publish in each matrix entry, so the most common partial-failure shape is "Open VSX accepted the new version, Marketplace rejected it". The fix is the same: bump version, re-tag. The previously-published Open VSX version remains accessible to users on `<previous>+1` until the next tag overwrites it; nothing user-visible breaks during the gap.

### Need to ship a fix

The semver-bump-and-tag procedure is the only path forward. There's no concept of "amending" a published release — bump to the next patch (or revert with a higher version if the broken release is in users' hands).

## Related

- The unified release workflow: [`.github/workflows/release.yml`](../.github/workflows/release.yml)
- Per-PR extension lint: [`.github/workflows/vscode-extension.yml`](../.github/workflows/vscode-extension.yml)
- Local-dev install paths for the extension: [`editor/vscode/DEVELOPING.md`](../editor/vscode/DEVELOPING.md)
