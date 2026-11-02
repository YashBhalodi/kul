# 13. Versioning policy

The Kul language is versioned by a `MAJOR.MINOR` scheme.

- **MINOR version increments** add backward-compatible features: new optional fields, new enumeration values, new statement kinds, additional sub-statement kinds. A document valid at version `0.MINOR` remains valid at `0.MINOR+1`.
- **MAJOR version increments** may make breaking changes: removing fields, renaming keywords, changing semantics. A `0.x` document is not guaranteed to be valid in `1.x`.

A Kul source carries no version declaration. A tool implements one version of this specification and reports it where it describes a project — the export envelope's `kul` field ([Section 15](./15-export-schema.md)). A tool implementing version `0.1` MUST parse and validate documents according to this specification. Because MINOR versions are additive, a document written for `0.1` stays valid for every tool implementing a later `0.x`.

---

← [Section 12 — Formal grammar](./12-formal-grammar.md) | [Index](./README.md) | Next → [Section 14 — Formatter rules](./14-formatter-rules.md)
