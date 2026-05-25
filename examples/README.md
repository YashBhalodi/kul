# Examples

Worked-example Kul projects that double as the **positive test corpus**. Each example is its own per-directory Kul project (one or more `.kul` files plus a sibling `kul.yml` manifest), and every example is exercised by integration tests in `crates/kul-core/tests/` and `crates/kul-cli/tests/` — they must always validate cleanly. If you add an example, the test suite will pull it in automatically; if you change one, snapshot tests will flag the diff.

| Project                                                                                                  | Demonstrates                                                                                                   |
| -------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| [`01-single-couple/single-couple.kul`](./01-single-couple/single-couple.kul)                             | Minimal shape: two persons + one ongoing marriage, no children.                                                |
| [`02-nuclear-family/nuclear-family.kul`](./02-nuclear-family/nuclear-family.kul)                         | A `birth` sub-statement deriving parents from a marriage.                                                      |
| [`03-three-generations/three-generations.kul`](./03-three-generations/three-generations.kul)             | Three generations, divorce (`end_reason`), retroactive adoption, circa date (`~1980`).                         |
| [`04-polygamous-family/polygamous-family.kul`](./04-polygamous-family/polygamous-family.kul)             | Two concurrent marriages for one person; child of one of them.                                                 |
| [`05-married-siblings/married-siblings.kul`](./05-married-siblings/married-siblings.kul)                 | Two sons of a couple, each themselves married; one block per marriage.                                         |
| [`06-three-branch-dynasty/three-branch-dynasty.kul`](./06-three-branch-dynasty/three-branch-dynasty.kul) | Three-branch dynasty: founders, three married children, four married grandchildren spread across the branches. |
| [`07-multi-file-extended-family/`](./07-multi-file-extended-family/)                                     | Multi-file project: three `.kul` files (founders / parents / grandchildren) sharing one project namespace with cross-file `birth` references. |
| [`08-divorce-and-remarriage/divorce-and-remarriage.kul`](./08-divorce-and-remarriage/divorce-and-remarriage.kul)                         | Divorce, both ex-spouses remarry with new children; past-marriage child-anchoring ghosts. |
| [`09-multi-adoption/multi-adoption.kul`](./09-multi-adoption/multi-adoption.kul)                         | Multiple adoptions for one child; the chain selects the most-recent as canonical and emits a child-ghost at each past adoption. |
| [`10-disconnected-lineages-and-orphan/disconnected-lineages-and-orphan.kul`](./10-disconnected-lineages-and-orphan/disconnected-lineages-and-orphan.kul) | Three disconnected components (two lineages plus an orphan) sorted in source order. |
| [`11-cousin-marriage/cousin-marriage.kul`](./11-cousin-marriage/cousin-marriage.kul)                     | First-cousin marriage exercising the within-family absorb rule. |
| [`12-polygamy-with-birth-family/polygamy-with-birth-family.kul`](./12-polygamy-with-birth-family/polygamy-with-birth-family.kul) | Pure-host polygamy embedded in a multi-generation tree; multiple concurrent marriages share one canonical card. |
| [`13-inter-family-marriage/inter-family-marriage.kul`](./13-inter-family-marriage/inter-family-marriage.kul) | Two unrelated birth families joined via a marriage; pure recursive nesting of the joining spouse's birth family adjacent to the host tree (the absorb rule). |
| [`15-polygamy-with-three-wives/polygamy-with-three-wives.kul`](./15-polygamy-with-three-wives/polygamy-with-three-wives.kul) | N=3 polygamy hub: three concurrent marriages on one person, one child per marriage. Exercises the fan rendering primitive (ADR-0020) at N=3 and per-marriage child attachment. |

## Layout

Each example is a self-contained Kul project: a numbered subdirectory of `examples/` carrying one or more `.kul` files plus a sibling `kul.yml` manifest. Single-file examples (01–06) keep the historical one-file-per-project shape; the multi-file example (`07-multi-file-extended-family/`) demonstrates the project-wide namespace landed by [ADR-0015](../docs/adr/0015-global-project-namespace.md) — every `.kul` file in the directory shares one logical namespace, with cross-file `birth` references resolving by bare id.

```
examples/
├── 01-single-couple/
│   ├── kul.yml
│   └── single-couple.kul
├── 02-nuclear-family/
│   ├── kul.yml
│   └── nuclear-family.kul
├── …
└── 07-multi-file-extended-family/
    ├── kul.yml
    ├── 01-founders.kul
    ├── 02-parents.kul
    └── 03-grandchildren.kul
```

## Try it

The CLI's `validate`, `format`, and `export` subcommands are CWD-rooted (issue #83) — each operates on the project rooted at the current working directory. From the repo root:

```sh
(cd examples/01-single-couple && kul validate)
(cd examples/07-multi-file-extended-family && kul validate)  # multi-file project
```

Every example must exit `0`. Anything that doesn't is either a regression or a new spec rule that hasn't been propagated to the corpus yet.

## Conventions

- Directory names: `NN-short-slug/`, where `NN` orders by complexity (smallest first).
- Single-file examples: the `.kul` file inside the directory drops the `NN-` prefix (e.g. `01-single-couple/single-couple.kul`).
- Multi-file examples: each `.kul` file is named by the slice of the project it carries, prefixed with `NN-` so alphabetic file order matches reading order (e.g. `01-founders.kul`, `02-parents.kul`); the directory itself carries the example number.
- Each `.kul` file starts with a header comment summarizing what it demonstrates.
- Each example is **self-contained** — every referenced ID is declared within that example's project directory (single file or sibling files in the same directory).
- Each example is a **happy path** — examples must validate cleanly. Negative fixtures live next to their tests in `crates/kul-core/tests/`, not here.

For the language reference, see [`spec/`](../spec/README.md). For domain vocabulary, see [`CONTEXT.md`](../CONTEXT.md).
