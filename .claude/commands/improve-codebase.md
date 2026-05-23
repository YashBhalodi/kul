---
description: Two-phase autonomous tech-debt sweep — orchestrator plans the full improvement list, then spawns one executor subagent per improvement, sequentially.
---

Run an autonomous tech-debt sweep on this Rust workspace. **Goal:** make future feature work easier to land. The user has no Rust expertise and is deferring all design judgment to you (the orchestrator) — bias **aggressive**: when a change would unlock future work, plan it; don't hedge to preserve the current shape. `just check` is the safety net.

**Two phases:**

- **Phase 1 (you plan).** You read the codebase, apply both skills' methodology, and produce a concrete, ordered list of improvements. All judgment happens here.
- **Phase 2 (executors apply).** For each improvement in order, you spawn a single-purpose subagent that receives **only** the instruction for that one change. It applies, verifies, commits, returns. You don't read or edit in this phase — you only orchestrate.

## Preflight (abort on any failure)

1. **Skills installed.** Both must exist:
   - `~/.claude/skills/improve-codebase-architecture/SKILL.md`
   - `~/.claude/skills/rust-skills/SKILL.md`

   If either is missing, print `ABORT: required skill <name> not installed at ~/.claude/skills/` and stop.

2. **Clean working tree.** `git status --porcelain` must be empty.

3. **Baseline `just check` green.** Run it once. If red, abort: `ABORT: just check fails on main — fix before sweeping.`

4. **Sync main** (no branch yet — we plan on main first, branch only if the plan is non-empty):

   ```sh
   git fetch origin
   git checkout main
   git pull --ff-only origin main
   ```

## Phase 1: Plan

You are reading and judging now. Use the `Explore` subagent for breadth so raw file contents don't pile up in your context — but you own the synthesis.

1. **Load both skills:** invoke `Skill(improve-codebase-architecture)` and `Skill(rust-skills)`.

2. **Read repo orientation:** `AGENTS.md`, `CONTEXT.md`, and list `docs/adr/` (titles only; read individual ADRs only when a candidate seems to touch them).

3. **Scout the codebase.** Use `Agent(subagent_type=Explore)` with breadth `"very thorough"`, one pass per lens (or combined if the agent can hold both):
   - **Architecture lens** — shallow modules, leaky seams, pass-throughs, untestable interfaces. Apply the deletion test.
   - **Rust-rules lens** — CRITICAL categories first (ownership, error handling, memory), then HIGH (API design, async, compiler-opt). The 179-rule catalog is in `~/.claude/skills/rust-skills/rules/`.

4. **Apply judgment.** You own the worth-it decision; the user has no Rust expertise to defer to. Rules of thumb:
   - Deletion test says a module is shallow? Plan to deepen it. "Works fine" is not a reason to drop.
   - Clean, contained Rust-rule fix? Include it.
   - **Implementation-coupled tests are themselves debt.** Per [`CODING_STANDARDS.md`](../../CODING_STANDARDS.md), tests should assert observable behavior through the public/`pub(crate)` interface — not private state, call order, or internal mocks. When you spot tests reaching into private fields, asserting on call sequences, or requiring visibility widening for the test to see internals, plan dedicated test-rewrite improvements even if no other refactor depends on them today. These tests will block future refactors; rewriting them now is high-leverage.
   - 50/50 on whether worth it? Include it. `just check` filters in Phase 2.
   - **ADR exception.** If a candidate contradicts a decision in `docs/adr/`, do **not** include it in the plan. Append it to `target/improve-codebase-skipped.log` (create `target/` first if needed) as `<area>: ADR-NNNN conflict — needs human revisit`. ADRs encode product/architecture intent the user did weigh in on; don't overturn autonomously.

5. **Order the list.** Foundational changes first (moving types, restructuring modules), then dependent improvements, then independent ones. A later improvement should not be invalidated by an earlier one if you can help it.

6. **Produce the plan.** A numbered list. Each entry must have:
   - **`subject`** — Conventional Commits subject line (`refactor:`, `perf:`, `fix:`, `test:`, `docs:`, etc.), ≤72 chars.
   - **`files`** — list of paths the executor will touch (best estimate).
   - **`instruction`** — concrete, executable description. State *what* changes and *how* in enough detail that a subagent with no other context can carry it out. Include snapshot/test expectations and any new ADRs or `CONTEXT.md` entries that should be added in the same commit.

   Keep instructions tight — no rationale, no "alternatives considered." The plan is for execution, not review.

7. **Empty plan?** Print `Codebase is clean. No improvements found.` and stop. Do not branch, do not push.

8. **Non-empty plan?** Now branch:

   ```sh
   git checkout -b improve-codebase/$(date +%Y-%m-%d-%H%M)
   ```

   Remember the branch name.

## Phase 2: Execute

For each entry in the plan, **in order**, spawn one `general-purpose` subagent (no worktree isolation — it commits on the real branch). Wait for it to return before spawning the next. Do not parallelize.

Subagent prompt (substitute `<branch>`, `<subject>`, `<files>`, `<instruction>`):

> You are a single-purpose executor on branch `<branch>`. Apply the change below, verify it, commit it. That's the whole job.
>
> **Scope rule.** Your scope is *exactly what's needed to land this refactor with `just check` green*. Inside that scope, tactical judgment calls are fine and expected — you're not a robot, you're an executor who can read the room. Outside that scope, hands off:
>
> - **In scope** (do it as part of this commit): the change itself; any test or doc updates the change requires; rewriting an implementation-coupled test that blocks the change per [`CODING_STANDARDS.md`](../../CODING_STANDARDS.md); fixing a compile error the change introduced.
> - **Out of scope** (leave it alone, even if you notice it): nearby debt that doesn't block this refactor; another impl-coupled test in a file this refactor doesn't touch; a different improvement you think is worth doing; "while I'm here" cleanups.
>
> If you find yourself doing something the orchestrator didn't ask for, ask: *does this refactor land without it?* If yes, you've left scope — stop and revert.
>
> **Commit subject:** `<subject>`
>
> **Files involved (estimate):** `<files>`
>
> **Instruction:**
>
> <instruction verbatim>
>
> **Steps:**
>
> 1. Read the involved files to understand the current state.
> 2. Apply the change exactly as instructed.
> 3. If the change shifts behavior covered by `insta` snapshots, accept the new snapshots (intentional snapshot diffs are fine; mention them in the commit body).
> 4. Run `just check` (fmt + clippy-deny-warnings + nextest).
> 5. **Green** → `git add` the touched paths and `git commit -m "<subject>"` with a one-sentence body explaining the *why*. Return `RESULT: applied`.
> 6. **Red** → diagnose the failure before deciding:
>    - **(a) Behavior actually changed.** A test correctly catches a behavior shift the instruction did not anticipate. `git reset --hard HEAD && git clean -fd`. Return:
>      ```
>      RESULT: skipped
>      REASON: <one-line: behavior the test catches that the change broke>
>      ```
>    - **(b) Test was coupled to the old implementation, not the behavior.** The failing test reaches into private state, asserts on call order/counts, depends on internal module structure, or otherwise checks *how* rather than *what*. Per [`CODING_STANDARDS.md`](../../CODING_STANDARDS.md) the test is the bug, not the refactor. Rewrite it to assert observable behavior through the public/`pub(crate)` interface, in the **same commit** as the refactor. Re-run `just check`. If green → commit and return `RESULT: applied`. If still red and the residual failure is type (a) → reset and skip per (a). If you can't reframe the test behaviorally without ambiguity → reset and return:
>      ```
>      RESULT: skipped
>      REASON: implementation-coupled test in <file> can't be reframed behaviorally without ambiguity
>      ```
>    - **(c) Failure isn't a test** (clippy, fmt, compile error). `git reset --hard HEAD && git clean -fd`. Return:
>      ```
>      RESULT: skipped
>      REASON: <one-line: what failed>
>      ```
>
>    Do not apply more than one round of test-rewriting per execution. If (b) leads to (a) leads to another (b), skip — the instruction needs human re-planning.
> 7. **Instruction stale** (files no longer exist, or the world shifted because of earlier commits in this sweep so the instruction no longer makes sense) → `git reset --hard HEAD && git clean -fd`. Return:
>    ```
>    RESULT: skipped
>    REASON: instruction stale — <one-line>
>    ```
>
> **You may consult `Skill(rust-skills)`** for Rust execution patterns. **Do not load `Skill(improve-codebase-architecture)`** — design judgment is already done; you are an executor.
>
> **Guardrails:** No `--no-verify`. No `#[allow(...)]` without an inline `// reason: ...` comment. No `--amend`, no force-push, no `git push`, no `gh pr create`. Don't expand scope beyond the instruction.
>
> **Return format.** End your final message with one of the two blocks above. The orchestrator parses only that; the rest is noise.

After each subagent returns:
- Parse the `RESULT:` line.
- If `skipped`, append `<subject>: <reason>` to `target/improve-codebase-skipped.log` (create the file if missing).
- Move to the next entry. Do not re-plan, do not retry.

## Phase 3: Finish

When the plan is exhausted:

1. `git log main..HEAD --oneline | wc -l` — if zero, delete the branch (`git checkout main && git branch -D <branch>`), print `All planned improvements were skipped. Nothing to push.`, and stop.

2. `git push -u origin HEAD`.

3. Build the PR body from on-disk state:
   - **Changes** — `git log main..HEAD --reverse --pretty=format:'- %s'`.
   - **Skipped** — bulleted contents of `target/improve-codebase-skipped.log` if non-empty. Omit the section if empty or missing.
   - Closing paragraph: this PR intentionally bundles multiple atomic improvements, overriding the `AGENTS.md` "one issue = one PR" convention for the sweep; reviewers should review commit-by-commit.

4. `gh pr create` with that body. Print the PR URL.

5. `rm -f target/improve-codebase-skipped.log`.

## Orchestrator scope reminders

- **Phase 1:** you read (directly + via Explore) and judge. You own the plan.
- **Phase 2:** you do not read source files, you do not edit, you do not pick or revise improvements. You only spawn one executor per plan entry, in order, and parse its `RESULT:` line.
- **Phase 3:** only `git` and `gh`.
- Don't bypass hooks, don't force-push, don't amend.
