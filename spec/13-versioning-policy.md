# 13. Versioning policy

The Kul language is versioned by a `MAJOR.MINOR` scheme.

- **MINOR version increments** add backward-compatible features: new optional fields, new enumeration values, new statement kinds, additional sub-statement kinds. A document valid at version `0.MINOR` remains valid at `0.MINOR+1`.
- **MAJOR version increments** may make breaking changes: removing fields, renaming keywords, changing semantics. A `0.x` document is not guaranteed to be valid in `1.x`.

A document with `kul 0.1` MUST be parsed and validated according to this specification. A parser encountering a higher version it does not know SHOULD report an error rather than silently parse the document under different rules.

---

← [Section 12 — Formal grammar](./12-formal-grammar.md) | [Section 14 — Formatter rules](./14-formatter-rules.md) | [Index](./README.md)
