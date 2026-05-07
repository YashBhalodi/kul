# Release process

KulLang ships four things from one repository: the `kul` CLI, the `kul-lsp` language server, the VSCode extension (published to Open VSX), and the `@kullang/wasm` npm package. They release in lockstep — one tag, one pipeline, one set of coordinated artifacts.

This doc is the source of truth for how to cut a release and what the pipeline does.

## Overview

Pushing a tag of the form `v<major>.<minor>.<patch>` triggers `.github/workflows/release.yml`, which:

1. **Verifies version coordination** — `Cargo.toml` workspace version, `editor/vscode/package.json` version, and the tag must all match. Fails fast if they don't. The `wasm-publish` job re-asserts the wasm-pack-produced npm `package.json` version against the same tag as a belt-and-braces guard.
2. **Builds `kul` for four targets** — `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`. Each binary is smoke-tested with `kul --version` and `kul validate examples/03-three-generations.kul`.
3. **Builds `kul-lsp` for the same four targets**, smoke-tested with `kul-lsp --version`.
4. **Builds and publishes `@kullang/wasm`** — `wasm-pack build --target bundler`, gzipped bundle-size assertion, `npm publish --access public`, and a `kul-wasm.tar.gz` artifact for the GitHub Release.
5. **Publishes the VSCode extension** to [Open VSX](https://open-vsx.org/) as four platform-specific `.vsix` files (one each for `darwin-arm64`, `darwin-x64`, `linux-x64`, `win32-x64`), each carrying the matching `kul-lsp` binary so end users don't need to set `kul.serverPath`. Each `.vsix` is also uploaded as a workflow artifact (`kul-vsix-<target>`) so the GitHub Release can attach it for upstream-VSCode users.
6. **Creates a GitHub Release** at `v<version>` with all 13 artifacts attached (8 CLI/LSP archives + 1 WASM tarball + 4 platform-specific `.vsix`) and auto-generated release notes.

`build-cli`, `build-lsp`, and `wasm-publish` run in parallel after `verify`; `openvsx-publish` runs as a 4-target matrix after `build-lsp` (each matrix entry bundles a single platform's LSP binary into a `--target`-tagged `.vsix`); `github-release` waits for all four (it consumes their artifacts). Total wall-clock time is dominated by the slowest matrix build.

```
verify ──┬──► build-cli   (4 targets) ──────────────────────────┐
         │                                                       │
         ├──► build-lsp   (4 targets) ──► openvsx-publish (4×) ──┼──► github-release   (13 artifacts)
         │                                       │               │
         │                                       └───────────────┴──► Open VSX (per-platform .vsix × 4)
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
- On an Open-VSX-consuming editor (VSCodium, Cursor, Windsurf, Theia/Che, Gitpod), `<editor> --install-extension YashBhalodi.kul` resolves against Open VSX, picks the matching platform `.vsix`, installs, and works without setting `kul.serverPath` (the extension auto-locates the bundled binary from `server/<platform>/`).
- On upstream VSCode (which talks to Microsoft Marketplace, where KulLang is intentionally not published), users install via the released `.vsix`: download the matching `kul-0.x.0-<target>.vsix` from the GitHub Release, `code --install-extension /path/to/kul-0.x.0-<target>.vsix`. Same bundled-binary autolocation behavior.
- `npm view @kullang/wasm version` returns `0.x.0`. A clean Node project can `npm install @kullang/wasm@0.x.0` and `import { check, exportGraph, format } from '@kullang/wasm'` without further setup.

### Recommended post-publish smoke

The integration tests cover protocol correctness, but only a human catches "the squiggle color is wrong on this theme" or "the hover popover is hard to read". After a release lands:

- Open each `examples/*.kul` in real VSCode (clean profile, bundled binary).
- Exercise diagnostics, hover, go-to-definition, completion on both light and dark themes.

## One-time setup

Before the very first Open VSX publish, the Eclipse account, namespace claim, and PAT must exist. Unlike the Microsoft Marketplace, no credit-card verification is required — an Eclipse account is free.

### a. Create the Eclipse account

- https://accounts.eclipse.org → Register (or sign in)
- Email verification only; no payment, no KYC

### b. Sign in to Open VSX and accept the Publisher Agreement

- https://open-vsx.org → "Log in" → GitHub OAuth
- Profile menu → user-settings → click "Show Publisher Agreement" and accept the terms
- The GitHub identity used for OAuth must be linked to (or share an email with) the Eclipse account

### c. Generate the publishing PAT

- https://open-vsx.org/user-settings/tokens → "Generate New Token"
- Description: anything memorable (e.g. `kul-release-ci`)
- Copy the token immediately (only shown once)

### d. Pre-claim the `YashBhalodi` namespace

The namespace name is case-sensitive — it must match `editor/vscode/package.json`'s `"publisher"` field. Open VSX does not auto-create namespaces on first publish, so this step must happen before tagging:

```sh
npx --yes ovsx create-namespace YashBhalodi --pat <token>
```

### e. Store as repo secret

```sh
gh secret set OVSX_PAT
# paste the token at the prompt
```

The publish job reads `OVSX_PAT` from secrets. Anytime the PAT expires or is rotated, repeat (c) and (e) — `ovsx publish` will start failing with a 401 until refreshed.

## Pipeline reference

### `verify`

Parses the version from the tag (or skips when triggered by `workflow_dispatch`), reads the workspace and extension versions, fails if any pair disagrees. Outputs the resolved version for downstream jobs.

### `build-cli` / `build-lsp`

Standard cross-compilation matrix. Each platform target builds in release mode with `Swatinem/rust-cache` for incremental speed. Smoke tests run on the platforms where they can — `x86_64-apple-darwin` is skipped because `macos-latest` is now ARM-based and would need Rosetta to execute.

`build-lsp` uploads two artifact sets per platform:

- An archive (`kul-lsp-<target>.{tar.gz,zip}`) for the GitHub Release.
- A raw binary under `kul-lsp-raw-<platform_dir>/` for the `openvsx-publish` job to bundle directly. This avoids re-downloading from a Release the workflow itself just produced.

### `wasm-publish`

Builds the `@kullang/wasm` package via `wasm-pack build --target bundler`, rewrites the wasm-pack-generated `package.json` `name` to `@kullang/wasm` (wasm-pack derives the npm name from the Rust crate name `kul-wasm`), asserts the gzipped `.wasm` is ≤ 1 MB, and re-asserts the npm `package.json` version equals the release version. The `pkg/` output is then staged into `kul-wasm/` and packaged as `kul-wasm.tar.gz`, uploaded as the `kul-wasm` artifact for `github-release` to attach to the public Release.

On a real publish (tag push or `dry_run: false`), the job also runs `npm publish --access public` from `crates/kul-wasm/pkg`, authenticated via the `NPM_TOKEN` repo secret. A pre-flight step fails with a readable error if `NPM_TOKEN` is unset, matching the `OVSX_PAT` failure shape. On dry-run, the build and the version assertions still run — only the npm publish is skipped, so dry-runs catch breakage before tagging.

The job does not re-run `cargo test`, the Node smoke, or `tsc --noEmit` — `.github/workflows/rust.yml`'s `wasm-build` job already gates the merge to `main`, so any commit a tag points at has already passed those checks.

### `github-release`

Pulls every archive artifact, copies them into a flat directory, and creates the public Release with `softprops/action-gh-release@v2`. Release notes are auto-generated from PRs/commits since the previous release.

### `openvsx-publish`

Runs as a 4-target matrix (`darwin-arm64`, `darwin-x64`, `linux-x64`, `win32-x64`). Each matrix entry pulls only its raw `kul-lsp` binary, stages it under `editor/vscode/server/<target>/`, `chmod +x`s it (belt-and-suspenders for vsce's zip layer), runs `npm ci`, and packages a platform-tagged extension via `vsce package --target <target>` (no global install — invoked through `npx @vscode/vsce` from the extension's `devDependencies`-resolved transitive). Each `.vsix` is uploaded as a workflow artifact named `kul-vsix-<target>` so `github-release` can attach it to the public Release.

On a real publish (tag push or `dry_run: false`), each matrix entry runs `npx ovsx publish kul-<version>-<target>.vsix -p $OVSX_PAT`. A pre-flight step fails with a readable error if `OVSX_PAT` is unset, matching the `NPM_TOKEN` failure shape. On dry-run, the package step and the artifact upload still run — only the OVSX publish is skipped, so dry-runs catch breakage before tagging.

Per-platform packaging is required: an "untagged" `.vsix` (no `--target`) is treated as platform-independent by Cursor's marketplace install path, which strips the `server/` directory on the assumption that platform-specific binaries are stale. The `--target` flag stamps `targetPlatform` into the manifest so the marketplace serves the matching `.vsix` to each user and preserves its bundled binary intact.

### `dry_run`

`workflow_dispatch` accepts a `dry_run` input (default `true`). When true, the conditional `if:` on `github-release` and on the `openvsx-publish` publish steps evaluates false, so the pipeline builds + smoke-tests every binary, packages the four `.vsix` files, and uploads the artifacts without publishing anything. Tag pushes always set `dry_run` effectively to false because the `if:` short-circuits on `github.event_name == 'push'`.

## Troubleshooting

### `verify` fails

The error message identifies which two of (workspace, extension, tag) disagree. Edit the offending file, re-tag if the tag was the wrong one (`git tag -d v0.x.0 && git push --delete origin v0.x.0` then re-tag).

### A `build-*` job fails

Look at the failing matrix entry. Most failures are a transient toolchain issue or a clippy/test regression — fix the underlying issue on `main`, push a new commit, and re-tag.

### `openvsx-publish` fails

- **`OVSX_PAT secret is unset`** — the pre-flight check ran. Set the secret per the one-time setup steps and re-run the failed job.
- **`401 Unauthorized` from `ovsx publish`** — the token expired, was revoked, or was generated against a different Eclipse account. Generate a fresh token at https://open-vsx.org/user-settings/tokens and `gh secret set OVSX_PAT`.
- **`Namespace 'YashBhalodi' does not exist`** — the namespace pre-claim was never run, or it was claimed under a different account. Run `npx --yes ovsx create-namespace YashBhalodi --pat <token>` once with the same token now stored in `OVSX_PAT`.
- **Secret-scanner rejection on upload** — Open VSX runs an automated scan and refuses uploads it flags. The error message identifies the offending file inside the `.vsix`. Fix at the source (typically a stray `.env`, key, or test fixture that landed in the bundled extension), bump and re-tag.
- **`Extension YashBhalodi.kul <version> already exists`** — `ovsx publish` is not idempotent on a published version. Bump the workspace version (and `editor/vscode/package.json`), re-tag, and re-run the pipeline.

### `wasm-publish` fails on the npm publish step

- **`NPM_TOKEN secret is unset`** — the pre-flight check ran. Set the secret per the one-time npm setup steps and re-run the failed job.
- **`E401`/`E403` from `npm publish`** — the token expired, was revoked, or lacks publish permission on the `@kullang` scope. Generate a fresh automation token at npmjs.com (Account → Access Tokens → Automation) and `gh secret set NPM_TOKEN`.
- **`E403 — cannot publish over existing version`** — the version was already published. `npm publish` is not idempotent; bump the workspace version (and `editor/vscode/package.json`), re-tag, and re-run the pipeline.
- **`E402 — payment required` on first publish** — the `@kullang` scope is private-by-default. The job already passes `--access public`; if this surfaces, the scope-claim setup wasn't completed (see issue #36's one-time npm setup section).

### Release exists but extension didn't publish

`github-release` depends on `openvsx-publish`, so an Open VSX publish failure on any of the four matrix entries blocks the Release from being created in the first place. If the OVSX failure was transient (network blip, registry hiccup), fix any underlying issue and re-run the failed jobs (Actions UI → workflow run → "Re-run failed jobs"); the per-platform `kul-vsix-<target>` artifacts uploaded earlier in the same job are reused. If Open VSX already accepted that version-and-target but the re-run mistakes it for a fresh attempt, you'll need to bump and re-cut — `ovsx publish` is not idempotent on the same version+target.

### Need to ship a fix

The semver-bump-and-tag procedure is the only path forward. There's no concept of "amending" a published release — bump to the next patch (or revert with a higher version if the broken release is in users' hands).

## Related

- The unified release workflow: [`.github/workflows/release.yml`](../.github/workflows/release.yml)
- Per-PR extension lint: [`.github/workflows/vscode-extension.yml`](../.github/workflows/vscode-extension.yml)
- Local-dev install paths for the extension: [`editor/vscode/README.md`](../editor/vscode/README.md)
