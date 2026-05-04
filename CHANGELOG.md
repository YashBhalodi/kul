# Changelog

All notable changes to the Kula CLI and core library are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — Unreleased

First public release of `kula-cli` and `kula-core`.

### Added

- **Parser + AST** for the full v1 surface of the Kula language: version
  declaration, `person` and `marriage` top-level statements, `birth` and
  `adoption` person sub-statements, all field types (string, gender,
  end-reason, date), date literals at three granularities (full, year-month,
  year-only) with optional `~` (circa) prefix.
- **Semantic analysis**: ID indexing across persons and marriages,
  reference resolution for marriage spouses and `birth`/`adoption` marriage
  refs, parent-graph queries.
- **Validator** implementing all 13 spec rules:
  - `KULA-R01` duplicate id; `KULA-R02` unresolved reference;
    `KULA-R03` required field missing; `KULA-R04` self-marriage;
    `KULA-R05` end-consistency (`KULA-R05b` for invalid `end_reason`).
  - Temporal: `KULA-R06` died-before-born; `KULA-R07`
    marriage-end-before-start; `KULA-R08` adoption-end-before-start;
    `KULA-R09` marriage-before-spouse-born; `KULA-R10`
    spouse-died-before-marriage; `KULA-R11`
    bio-child-born-before-parent; `KULA-R12`
    adoption-before-adopter-born.
  - Cycles: `KULA-R13` parenthood cycle (iterative DFS, O(V+E)).
- **CLI `kula validate`**:
  - Multiple files in one invocation.
  - `-` reads from stdin.
  - `--quiet` suppresses success output.
  - `--format json` emits one JSON object per diagnostic (jsonl).
  - `--no-color` forces colorless output.
  - Exit code `0` on success, `1` on any error diagnostic.
- Cross-platform release workflow producing binaries for
  `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`,
  and `x86_64-pc-windows-msvc`.
