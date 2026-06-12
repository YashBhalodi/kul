---
description: Cut and push a new KulLang release — investigate scope since the last tag, recommend a version, bump + changelog in one atomic commit on main, then tag and push to trigger the pipeline. Sole source of truth for the release runbook.
---

Cut a new release of KulLang. This command is the **sole source of truth** for the release runbook — there is no `docs/release.md`.

The release **pipeline** is defined by `.github/workflows/release.yml` — that file, not this one, is the truth for what happens after a tag is pushed. **Read it** at the start of every release rather than trusting any summary here; it tells you which files must carry the version, what gets built, and what gets published. In essence: pushing a tag `v<major>.<minor>.<patch>` makes the workflow verify the version, build the binaries, publish the npm/extension/wasm artifacts in lockstep, and create a GitHub Release. **None of the registries (npm, Open VSX, VS Code Marketplace) is idempotent on a version**, so a tag is irreversible — a bad one is recovered only by bumping to the next version.

You will: investigate → recommend a version + draft the changelog(s) → get **one approval** → then commit, push to main, tag, and push the tag autonomously, reporting the run URL. A single dry-run question is folded into that approval.

## Preflight (abort on any failure)

- **On `main`, clean, in sync with origin** — `git status --porcelain` empty, `main` checked out, `git fetch` then HEAD == upstream. Releases cut from `main` only.
- **`gh auth status`** succeeds (needed to trigger and watch the workflow).
- **Versions already agree.** Read `release.yml`'s `verify` job to learn which files it requires to carry the version and match the tag, then read those files. They must all be equal *before you start*; call that value `CURRENT`. If they drift, stop — that's a pre-existing bug to fix first. Confirm `CURRENT` equals the latest tag (`git describe --tags --abbrev=0` → `v<CURRENT>`); if it's already ahead, a prior release may be half-cut — investigate.

## Phase 1 — Investigate the scope

Enumerate everything since the last tag — `git log v<CURRENT>..HEAD` and `git diff v<CURRENT>..HEAD --stat` — and read the actual diffs/PRs (don't trust commit subjects). For each change, note which shipped component it touches and whether it's a fix, a feature, or internal-only. Flag anything release-significant: a schema-version bump (the strongest signal toward a higher version), and whether the Kul **language** version is affected — it almost never is, since the toolchain versions independently; say so explicitly ("documents valid at `<CURRENT>` remain valid") when true. A pure lockstep bump with no user-visible change is a legitimate release.

## Phase 2 — Recommend the version + draft the changelog

1. **Recommend `NEXT`** by semver, pre-1.0 (`0.x`): **patch** for fixes / internal / docs / lockstep-only; **minor** for a notable batch of new user-facing capability or any render/export/wasm **schema** bump; **major** only on explicit direction (1.0). Calibrate against recent cadence (`git tag`). Give the one-line reason.
2. **Draft the changelog(s).** Find every changelog in the repo that carries an `[Unreleased]` section (at least the root `CHANGELOG.md`; there may be a per-extension one). For each: promote `[Unreleased]` to `## [<NEXT>] — <today>` (`date +%Y-%m-%d`) and add a fresh empty `[Unreleased]` above it. **Match the surrounding entries' style exactly** — read the last release's entry and mirror its structure, headings, and PR-reference convention; compose missing content from your Phase 1 analysis. Touch `README.md` only if it carries a real *toolchain* version reference (its `0.1` is the **language** version — leave it).
3. **Compose the commit message** — subject `chore: release v<NEXT>`, a body summarizing the release in the shape of the previous release commit (`git show` the last `chore: release` commit and mirror it), and the trailer `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
4. **One approval gate.** Present together: `NEXT` + reason, the drafted changelog entries, the commit message, and the **dry-run question** — "Run the dry-run pipeline before tagging? Builds + packages everything on every platform with no publish, catching a break before the irreversible tag, at the cost of extra wall-clock." Apply any edits they ask for, then proceed without further prompts unless a step fails.

## Phase 3 — Execute (autonomous after approval)

1. **Bump** every version file the `verify` job named, to `NEXT`.
2. **Regenerate whatever lockfiles the bump touches** so none is left stale — `cargo check --workspace` for `Cargo.lock`, `npm install` (repo root) for `package-lock.json`, and any other the bump affects. Then `git diff` and confirm the change is only version fields, lockfile version lines, and the changelog(s) — if anything else moved, pause and surface it.
3. **Commit** all of it as one atomic `chore: release v<NEXT>` with the approved message; confirm a clean tree after.
4. **Push to `main`** — `git push origin main`.
5. **Dry-run gate** (per the approval answer). If yes: trigger it against the just-pushed HEAD (the workflow exposes a `dry_run` input — `gh workflow run release.yml --ref main -f dry_run=true`; if the input shape changed, read the workflow) and watch to completion (`gh run watch`). On failure, **stop and do not tag** — report which job broke; the fix is a new commit on `main`, then re-run this command. If no, continue.
6. **Tag and push** — `git tag v<NEXT>` on the bump commit, `git push origin v<NEXT>`. The tag must be exactly `v<major>.<minor>.<patch>` or the workflow's tag filter and `verify` reject it.
7. **Report** the Actions URL and the Release URL for `v<NEXT>`. Optionally `gh run watch` the tag-triggered run. "Done" = the GitHub Release exists with its artifacts and the new version is live on npm and both extension registries.

## One-time setup (only before the first release, or when a token expires)

The pipeline publishes via three repo secrets — `OVSX_PAT` (Open VSX), `VSCE_PAT` (VS Code Marketplace), `NPM_TOKEN` (npm `@kullang` scope) — set with `gh secret set <NAME>`. After the first release the only maintenance is rotating an expired token (a `401`/`E401`/`E403` in the matching publish step is the tell). First-time-only specifics that aren't error-recoverable:

- **Open VSX** needs the `YashBhalodi` namespace pre-claimed once (it does not auto-create): `npx --yes ovsx create-namespace YashBhalodi --pat <token>`. The namespace name is case-sensitive and must equal the extension manifest's `publisher`.
- **VS Code Marketplace** needs a publisher `ID = YashBhalodi` created at the Marketplace, and a PAT scoped to **Marketplace → Manage** for **All accessible organizations** (org-scoped tokens fail).
- **npm** needs an Automation token with publish rights on the `@kullang` scope; the job already passes `--access public` (the scope is private by default).

## Troubleshooting

The pipeline's job logs name the failing step; fix forward from there. The one rule that governs almost every published-registry failure: **no registry is idempotent on a version** — you cannot retry a half-published tag, so the recovery is always *bump to the next patch, re-tag, re-run this command*. The dry-run gate exists to catch build/package breaks before that irreversible tag. Specifics worth knowing:

- **`verify` fails** — the error names which version files (or the tag) disagree. If the *tag* was wrong, delete it (`git tag -d v<NEXT> && git push --delete origin v<NEXT>`) and re-tag; if a *file* was wrong, that's a new commit on `main`.
- **A publish step fails on auth** (`401`/`E401`/`E403`, "PAT verification failed", "namespace/publisher does not exist") — the matching token expired or lacks scope, or the one-time setup above wasn't done. Regenerate/redo and `gh secret set`.
- **`<version> already exists`** in any registry — that registry took it; the version is burned. Bump and re-cut. The extension publishes Open-VSX-first, so the common partial failure is "Open VSX accepted, Marketplace rejected" — same fix; the prior Open VSX version stays live for users in the gap.
- **A transient build/network failure** can be re-run from the Actions UI ("Re-run failed jobs") *only* if no registry accepted the version yet; once one did, re-running is mistaken for a fresh publish and fails — bump and re-cut.
