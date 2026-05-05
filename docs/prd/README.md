# PRDs

Product Requirements Documents for KulaLang. Each PRD captures the scope, user stories, and implementation/testing decisions for a single product epic — at the level a contributor (human or AI) needs to understand "why are we building this, and what does done look like?"

## Lifecycle

PRDs are **transient assets**. Each one is meant to be deleted from this directory once its product epic is fully implemented and shipped. They are not historical records — they are working documents that exist only as long as their epic is in flight.

When you finish implementing a PRD, delete the file in the same PR as the final piece of implementation work. If a decision inside the PRD is load-bearing enough to outlive the epic — a contract a future agent might re-propose changing — lift it into an ADR before deleting the PRD.

The persistent record of what was built and why lives elsewhere:

- The **language and runtime contract** lives in [`spec/`](../../spec/).
- The **non-obvious load-bearing decisions** live in [`docs/adr/`](../adr/), immortal and superseded only by new ADRs.
- The **change history** lives in `git log` and [`CHANGELOG.md`](../../CHANGELOG.md).

## Naming

`NNNN-kebab-short-title.md` with the next free `NNNN`, mirroring the ADR convention for visual consistency. Numbers are not reused after a PRD is deleted; the next PRD takes the next free number.

## Relationship to issues

A PRD lives in this directory; its implementation work lives as one or more GitHub issues that reference the PRD. Issues come and go as work proceeds; the PRD stays until the epic is done, then it goes too.
