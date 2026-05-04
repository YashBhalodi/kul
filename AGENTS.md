# AGENTS.md

Conventions and configuration for AI agents working in this repository. Read this on entry.

## Agent skills

### Issue tracker

GitHub Issues at `YashBhalodi/kulalang` via the `gh` CLI. See [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md).

### Triage labels

The five canonical triage roles use their default label strings (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md).

### Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root (neither file exists yet — the `grill-with-docs` and `improve-codebase-architecture` skills will populate them lazily as the implementation begins). See [`docs/agents/domain.md`](./docs/agents/domain.md).
