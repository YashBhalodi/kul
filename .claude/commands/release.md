---
description: Cut and push a new KulLang release — investigate scope since the last tag, recommend a version, bump + changelog in one atomic commit on main, then tag and push to trigger the pipeline. Sole source of truth for the release runbook.
---

Cut a new release of KulLang. This command is the **sole source of truth** for the release runbook — there is no `docs/release.md`.

The release **pipeline** is defined by `.github/workflows/release.yml` — that file, not this one, is the truth for what happens after a tag is pushed. **Read it** at the start of every release rather than trusting any summary here. In essence: pushing a tag `v<major>.<minor>.<patch>` makes the workflow check the version lockstep (`scripts/release-versions.mjs`), build and smoke-test the `kul` CLI for every platform, and create a GitHub Release carrying those archives and generated release notes. Nothing goes to a package registry. The web app is not part of a release: it deploys from `main` through its own workflow, so every merge ships it.

A release tag is **permanent**: the "Protect release tags" ruleset blocks deleting or moving any `v*` tag. A bad tag is recovered only by bumping to the next version.

You will: investigate → recommend a version + draft the changelog → get **one approval** → then commit, push to main, tag, and push the tag autonomously, reporting the run URL. A single dry-run question is folded into that approval.

## Preflight (abort on any failure)

- **On `main`, clean, in sync with origin** — `git status --porcelain` empty, `main` checked out, `git fetch` then HEAD == upstream. Releases cut from `main` only.
- **`gh auth status`** succeeds (needed to trigger and watch the workflow).
- **Versions already agree.** Run `node scripts/release-versions.mjs`; it prints the version every lockstep file carries, or names the files that disagree. Call that value `CURRENT`. If they drift, stop — that's a pre-existing bug to fix first. Confirm `CURRENT` equals the latest tag (`git describe --tags --abbrev=0` → `v<CURRENT>`); if it's already ahead, a prior release may be half-cut — investigate.
- **No stale secrets.** The pipeline needs no repository secret (the GitHub Release uses the built-in `GITHUB_TOKEN`). Compare `gh secret list` with the secrets the workflows reference (`rg -o 'secrets\.[A-Z_]+' .github/workflows | sort -u`). Report any secret no workflow references so the maintainer can delete it (`gh secret delete <NAME>`); don't delete it yourself.

## Phase 1 — Investigate the scope

Enumerate everything since the last tag — `git log v<CURRENT>..HEAD` and `git diff v<CURRENT>..HEAD --stat` — and read the actual diffs/PRs (don't trust commit subjects). For each change, note which part it touches (the CLI, the web app, the language, the docs) and whether it's a fix, a feature, or internal-only. Flag anything release-significant: an export-schema bump (the strongest signal toward a higher version), and whether the Kul **language** version is affected — it almost never is, since the toolchain versions independently; say so explicitly ("documents valid at `<CURRENT>` remain valid") when true. Web-app changes since the last tag are already live; the changelog still records them under the new version. A pure version bump with no user-visible change is a legitimate release.

## Phase 2 — Recommend the version + draft the changelog

1. **Recommend `NEXT`** by semver, pre-1.0 (`0.x`): **patch** for fixes / internal / docs; **minor** for a notable batch of new user-facing capability or any export **schema** bump; **major** only on explicit direction (1.0). Calibrate against recent cadence (`git tag`). Give the one-line reason.
2. **Draft the changelog.** In `CHANGELOG.md`, promote `[Unreleased]` to `## [<NEXT>] — <today>` (`date +%Y-%m-%d`) and add a fresh empty `[Unreleased]` above it. **Match the surrounding entries' style exactly** — read the last release's entry and mirror its structure, headings, and PR-reference convention; compose missing content from your Phase 1 analysis. Touch `README.md` only if it carries a real *toolchain* version reference (its `0.1` is the **language** version — leave it).
3. **Compose the commit message** — subject `chore: release v<NEXT>`, a body summarizing the release in the shape of the previous release commit (`git show` the last `chore: release` commit and mirror it), ending with the attribution trailers your session requires.
4. **One approval gate.** Present together: `NEXT` + reason, the drafted changelog entry, the commit message, and the **dry-run question** — "Run the dry-run pipeline before tagging? Builds and smoke-tests the CLI on every platform without creating a release, catching a break before the permanent tag, at the cost of extra wall-clock." Apply any edits they ask for, then proceed without further prompts unless a step fails.

## Phase 3 — Execute (autonomous after approval)

1. **Bump** every version `scripts/release-versions.mjs` checks, to `NEXT`: the workspace `Cargo.toml`'s `[workspace.package] version` (and the `version` of each internal crate in its `[workspace.dependencies]`) and `apps/web/package.json`'s `version`. Run `node scripts/release-versions.mjs --tag v<NEXT>` and confirm it prints `NEXT`.
2. **Regenerate the lockfiles** so none is left stale — `cargo check --workspace` for `Cargo.lock`; `just wasm` then `npm install` at the repo root for `package-lock.json` (the `kul-wasm` workspace package takes its version from the crate). Then `git diff` and confirm the change is only version fields, lockfile version lines, and the changelog — if anything else moved, pause and surface it.
3. **Commit** all of it as one atomic `chore: release v<NEXT>` with the approved message; confirm a clean tree after.
4. **Push to `main`** — `git push origin main`.
5. **Dry-run gate** (per the approval answer). If yes: trigger it against the just-pushed HEAD — `gh workflow run release.yml --ref main -f dry-run=true` (if the input shape changed, read the workflow) — and watch to completion (`gh run watch`). On failure, **stop and do not tag** — report which job broke; the fix is a new commit on `main`, then re-run this command. If no, continue.
6. **Tag and push** — `git tag v<NEXT>` on the bump commit, `git push origin v<NEXT>`. The tag must be exactly `v<major>.<minor>.<patch>` or the workflow's tag filter and `verify` reject it.
7. **Report** the Actions URL and the Release URL for `v<NEXT>`. Optionally `gh run watch` the tag-triggered run. "Done" = the GitHub Release for `v<NEXT>` exists with one CLI archive per platform and its release notes.

## Troubleshooting

The pipeline's job logs name the failing step; fix forward from there. The tag is permanent, so the recovery for anything past the tag push is *fix on `main`, bump to the next patch, re-run this command*. The dry-run gate exists to catch build breaks before the tag. Specifics worth knowing:

- **`verify` fails** — its error annotations name the files (or the tag) that disagree. The tag can't be deleted, so fix the files in a new commit on `main` and release the next patch.
- **A `build-cli` target fails** — no release was created (`github-release` needs every target). If the failure is transient (runner or network), re-run the failed jobs from the Actions UI; the tag-triggered run then completes the release. If it's a real break, fix it on `main` and release the next patch.
- **`github-release` fails** — the CLI archives are still attached to the run as artifacts. Re-running the failed job retries the release for the same tag; there is no registry, so nothing is burned by a partial attempt.
