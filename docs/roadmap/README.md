# KulaLang Roadmap

This folder contains the PRDs (Product Requirement Documents) that take KulaLang from "v0.1 spec done, no implementation" to **a working VSCode experience**: a user creates a `.kula` file, the official Kula extension activates, and they get syntax highlighting, autocomplete, hover, go-to-definition, and live error diagnostics.

The implementation is split into four phases. Each phase is independently shippable and produces user-visible value on its own.

## Cross-cutting constraints

Two constraints apply to every phase:

1. **Zero-Rust-experience contributor.** The repo owner has not written Rust before. All Rust choices favor conventional, well-trodden tooling (`cargo`, `clippy`, `rustfmt`, `cargo-nextest`) over novel options. Code structure and idioms should be discoverable by reading the project, not by knowing Rust trivia.
2. **AI-agent autonomous development DX.** The implementation must give an AI agent (Claude Code or similar) high-quality feedback loops so it can take a feature, build it, verify it, and iterate without human intervention. This means: fast tests (seconds, not minutes), crisp error output with line/column anchors, a single `just check` command that runs all gates, golden test files for every spec rule, and snapshot-based assertions where appropriate.

These two constraints often pull the same direction: well-trodden Rust tooling has the best documentation and the best agent-readable error output.

## Phases

| #  | PRD                                                          | Headline deliverable                                                          | Status      |
| -- | ------------------------------------------------------------ | ----------------------------------------------------------------------------- | ----------- |
| 1  | [VSCode extension (TextMate-only)](./01-vscode-extension.md) | Published `.vsix` with syntax highlighting, snippets, file icon. No LSP yet.  | Not started |
| 2  | [Parser, validator, CLI](./02-parser-validator-cli.md)       | `kula validate <file>` reports the 13 spec errors with line/col anchors.      | Not started |
| 3  | [Basic LSP](./03-basic-lsp.md)                               | Live diagnostics, hover, go-to-def, keyword/field/enum completion in VSCode.  | Not started |
| 4  | [Polished LSP](./04-polished-lsp.md)                         | ID-aware completion, find references, rename, formatter, document symbols.   | Not started |

## Why this phasing

Phase 1 delivers immediate user value (colors + snippets) and forces the marketplace publishing ceremony while the artifact is small and low-stakes.

Phase 2 produces the *substance* — parser, AST, validator — that everything else consumes. A working CLI is a forcing function for a clean library API; the LSP later just calls into the same code with a different consumer.

Phase 3 is the headline release: real-time editor feedback. After this, Kula "feels like a real language" in VSCode.

Phase 4 is incremental polish — each feature is shippable on its own.

## What's NOT in this roadmap

- **Web visualization app.** Explicitly downstream per [`../vision.md`](../vision.md).
- **Multi-file / import support.** Deferred to a future spec version (v0.2+).
- **Kinship query language** (deriving siblings, cousins, etc.). A separate "queries / views" project that sits on top of the AST.
- **Editor-specific extensions for vim / emacs / helix / zed.** The LSP works in any of them; we document "point your LSP client at the `kula-lsp` binary" but don't ship dedicated extensions.
- **Live family-tree visualization in VSCode.** Could be a later VSCode webview but it's out of the v1 envelope.

## How to consume these PRDs

Each PRD is the source of truth for its phase. When implementing a phase, the implementer (human or AI agent) reads the PRD, breaks it into tasks (see the "User Stories" section, which functions as a story breakdown), and works through them. The "Out of Scope" section is binding — features called out as out-of-scope for a phase do not get smuggled in.

When a phase ships, mark its row in the table above with the published version (e.g. "Shipped — extension v0.0.1") and link to the release.
