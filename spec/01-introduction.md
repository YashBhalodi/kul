# 1. Introduction

Kul is a domain-specific language for describing human kinship — the structure of families and how they evolve over time. A Kul document is a plain UTF-8 text file with the `.kul` extension. The contents describe persons and the marriages between them; biological parenthood and adoption are recorded as references inside person declarations.

The two primitives of Kul are:

- **Person** — an identifiable individual.
- **Marriage** — a temporal binary union between two persons.

Parenthood (biological and adoptive) is _not_ a separate primitive. It is represented as references on a Person, pointing to the Marriage that produced them (biological birth) or admitted them as a child (adoption).

Out of scope for v1: non-marriage romantic partnerships, sperm donors and surrogates, single parenthood, polyamorous co-parenting (more than two parents in the same parenthood unit), friendships, professional relationships, location and biographical data, multi-file documents, cultural prohibitions on marriage. See [`../docs/vision.md`](../docs/vision.md) for the full scope statement.

---

[Index](./README.md) | Next → [Section 2 — Document structure](./02-document-structure.md)
