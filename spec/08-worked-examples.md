# 8. Worked examples

The four documents in [`../examples/`](../examples/) progressively exercise the language. Each is a complete, valid Kul 0.1 document and serves as both a tutorial and a test corpus for tooling.

## 8.1 A single couple

[`../examples/01-single-couple/single-couple.kul`](../examples/01-single-couple/single-couple.kul)

A married couple, no children, marriage ongoing. Both persons are documentation roots (no `birth` sub-statement). Exercises: `person`, `marriage`, ongoing marriage (no `end`).

## 8.2 A nuclear family

[`../examples/02-nuclear-family/nuclear-family.kul`](../examples/02-nuclear-family/nuclear-family.kul)

Carol is the biological child of Alice and Bob's marriage. Exercises: `birth` sub-statement; biological parenthood derived from a marriage's spouses.

## 8.3 A three-generation family with adoption

[`../examples/03-three-generations/three-generations.kul`](../examples/03-three-generations/three-generations.kul)

Three generations. Alice is the biological daughter of Ramesh and Sita. Alice and Bob have a biological daughter Carol and an adopted son Ravi. The Alice-Bob marriage ends in divorce; Bob later dies. Exercises: multi-generation references, `adoption` sub-statement, `end`/`end_reason`, `died`, circa dates (`born:~1980`), comments and section headers.

## 8.4 A polygamous family

[`../examples/04-polygamous-family/polygamous-family.kul`](../examples/04-polygamous-family/polygamous-family.kul)

Devraj is concurrently married to Meera and Alice. Priya is the biological daughter of Alice and Devraj. Exercises: concurrent marriages for one spouse (polygamy); cross-family naming.

## 8.5 Host effect across a multi-family chain

[`../examples/05-married-siblings/married-siblings.kul`](../examples/05-married-siblings/married-siblings.kul)

Two sons (Arjun, Vikram), each born of `m_ramesh_sita`, themselves marry into other families. The two child-marriages list the Sharma-born sibling first:

```
marriage m_arjun_priya  arjun  priya   start:1975-11-04
marriage m_vikram_nisha vikram nisha   start:1980-03-22
```

Per [Section 4.2](./04-top-level-statements.md#42-marriage-statement), `arjun` and `vikram` are the **hosts** of their respective marriages; `priya` and `nisha` join. The host position is what threads the founders' marriage `m_ramesh_sita` through to each child-marriage as one continuous structural chain (parent marriage → host → child marriage). Swapping the spouse identifiers in either marriage would make the in-law the host, breaking the chain at that branch — visible to any layout consumer that follows the host edge. Exercises: deliberate host choice; multi-family layout consequence.

---

← [Section 7 — Validation rules](./07-validation-rules.md) | [Index](./README.md) | Next → [Section 9 — Edge cases](./09-edge-cases.md)
