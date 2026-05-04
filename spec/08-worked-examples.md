# 8. Worked examples

The four documents in [`../examples/`](../examples/) progressively exercise the language. Each is a complete, valid Kula 0.1 document and serves as both a tutorial and a test corpus for tooling.

## 8.1 A single couple

[`../examples/01-single-couple.kula`](../examples/01-single-couple.kula)

A married couple, no children, marriage ongoing. Both persons are documentation roots (no `birth` sub-statement). Exercises: `person`, `marriage`, ongoing marriage (no `end`).

## 8.2 A nuclear family

[`../examples/02-nuclear-family.kula`](../examples/02-nuclear-family.kula)

Carol is the biological child of Alice and Bob's marriage. Exercises: `birth` sub-statement; biological parenthood derived from a marriage's spouses.

## 8.3 A three-generation family with adoption

[`../examples/03-three-generations.kula`](../examples/03-three-generations.kula)

Three generations. Alice is the biological daughter of Ramesh and Sita. Alice and Bob have a biological daughter Carol and an adopted son Ravi. The Alice-Bob marriage ends in divorce; Bob later dies. Exercises: multi-generation references, `adoption` sub-statement, `end`/`end_reason`, `died`, circa dates (`born:~1980`), comments and section headers.

## 8.4 A polygamous family

[`../examples/04-polygamous-family.kula`](../examples/04-polygamous-family.kula)

Devraj is concurrently married to Meera and Alice. Priya is the biological daughter of Alice and Devraj. Exercises: concurrent marriages for one spouse (polygamy); cross-family naming.

---

← [Section 7 — Validation rules](./07-validation-rules.md) | [Index](./README.md) | Next → [Section 9 — Edge cases](./09-edge-cases.md)
