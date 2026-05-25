# 8. Worked examples

The projects in [`../examples/`](../examples/) progressively exercise the language. Each is a complete, valid Kul 0.1 project and serves as both a tutorial and a test corpus for tooling. This section highlights the language constructs each one introduces; see [`../examples/README.md`](../examples/README.md) for the full annotated list.

## 8.1 A nuclear family

[`../examples/01-nuclear-family/nuclear-family.kul`](../examples/01-nuclear-family/nuclear-family.kul)

A married couple with two children. Exercises the three core constructs: `person`, `marriage`, and the `birth` sub-statement that derives a child's parents from a marriage's spouses.

## 8.2 Three generations

[`../examples/02-three-generations/three-generations.kul`](../examples/02-three-generations/three-generations.kul)

Three generations of one family. Exercises multi-generation `birth` references, the optional `family:` / `given:` fields, the full range of date precision (`YYYY-MM-DD`, `YYYY-MM`, bare `YYYY`, and the `~` circa prefix), and `died:` — which records a death without ending the deceased's marriage (only a marriage's own `end:` does that).

## 8.3 Divorce and remarriage

[`../examples/03-divorce-and-remarriage/divorce-and-remarriage.kul`](../examples/03-divorce-and-remarriage/divorce-and-remarriage.kul)

A couple divorces and each remarries. Exercises `end:` with its required `end_reason:`, and a person participating in more than one marriage over time.

## 8.4 Adoption

[`../examples/04-adoption-and-belonging/adoption-and-belonging.kul`](../examples/04-adoption-and-belonging/adoption-and-belonging.kul)

A family built largely by adoption. Exercises the `adoption` sub-statement with its `start:` (and optional `end:`), a person carrying both a `birth` and multiple `adoption`s at once, and `gender:other`.

## 8.5 Host effect across a multi-family chain

[`../examples/05-cousins-and-in-laws/cousins-and-in-laws.kul`](../examples/05-cousins-and-in-laws/cousins-and-in-laws.kul)

An extended family in which one spouse marries in from another family and, later, two cousins marry. Per [Section 4.2](./04-top-level-statements.md#42-marriage-statement), the first-listed spouse of each marriage is the **host** and the second joins the host's family. The host position is what threads a parent marriage through to a child marriage as one continuous structural chain (parent marriage → host → child marriage); swapping the spouse identifiers in a marriage moves the host, and a layout consumer that follows the host edge sees the difference. Exercises deliberate host choice and its multi-family consequence.

## 8.6 Concurrent marriages

[`../examples/06-polygamous-household/polygamous-household.kul`](../examples/06-polygamous-household/polygamous-household.kul)

One person concurrently married to three others. Exercises multiple un-ended marriages for a single spouse (polygamy) and rule [R14](./07-validation-rules.md): a person with two or more un-ended marriages must be the host of every one of them.

## 8.7 Unrelated families and an orphan

[`../examples/07-disconnected-lineages/disconnected-lineages.kul`](../examples/07-disconnected-lineages/disconnected-lineages.kul)

Several families that share no relatives, plus a person declared with no ties at all. Exercises a single document holding multiple disconnected components, and a person who is neither a spouse nor a child.

## 8.8 A multi-file project

[`../examples/08-multi-file-project/`](../examples/08-multi-file-project/)

One family split across three `.kul` files in one project directory. Exercises the project-wide flat namespace ([ADR-0015](../docs/adr/0015-global-project-namespace.md)): every id is visible from every file by bare name, with no imports, and `birth` lines resolve marriages declared in sibling files.

## 8.9 A family across a century

[`../examples/09-family-across-a-century/family-across-a-century.kul`](../examples/09-family-across-a-century/family-across-a-century.kul)

A ~30-person dynasty that combines every construct above — widowhood, concurrent marriages, divorce and remarriage, adoption, marrying-in from other families, mixed date precision, and `gender:other` — in one realistic document.

---

← [Section 7 — Validation rules](./07-validation-rules.md) | [Index](./README.md) | Next → [Section 9 — Edge cases](./09-edge-cases.md)
