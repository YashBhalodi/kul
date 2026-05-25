# Examples

Worked-example Kul projects that double as the **positive test corpus**. Read top to bottom, they're a guided tour of the whole language — each builds on the last, from the smallest complete document to a full multi-generation dynasty. Each example is its own per-directory Kul project (one or more `.kul` files plus a sibling `kul.yml` manifest), and every one is exercised by integration tests across the workspace — they must always validate cleanly. Add an example and the test suite pulls it in automatically; change one and snapshot tests flag the diff.

| Project | Demonstrates |
| ------- | ------------ |
| [`01-nuclear-family/`](./01-nuclear-family/nuclear-family.kul) | The three core constructs — `person`, `marriage`, and the `birth` sub-statement: a couple and their two children. |
| [`02-three-generations/`](./02-three-generations/three-generations.kul) | Generational depth; the full range of date precision (`YYYY-MM-DD` / `YYYY-MM` / `YYYY` / circa `~`); `family:` / `given:`; and `died:`, which records a death without ending the marriage (a widow stays in her family). |
| [`03-divorce-and-remarriage/`](./03-divorce-and-remarriage/divorce-and-remarriage.kul) | `end:` / `end_reason:`; a person in more than one marriage over time; a blended family whose first marriage is held in place by ghosts. |
| [`04-adoption-and-belonging/`](./04-adoption-and-belonging/adoption-and-belonging.kul) | The `adoption` sub-statement (with `start:` / `end:`); a person carrying both a `birth` and several adoptions; adopted and biological siblings as equal members; `gender:other`. |
| [`05-cousins-and-in-laws/`](./05-cousins-and-in-laws/cousins-and-in-laws.kul) | The host rule shown both ways in one extended family: a spouse who marries in from another family, and a marriage between two cousins already in the tree. |
| [`06-polygamous-household/`](./06-polygamous-household/polygamous-household.kul) | Concurrent marriages: one person married to three others at once, and rule R14 (the hub must host every concurrent marriage). |
| [`07-disconnected-lineages/`](./07-disconnected-lineages/disconnected-lineages.kul) | Several unrelated families in one document, plus a lone individual with no ties — and how source order arranges them. |
| [`08-multi-file-project/`](./08-multi-file-project/) | A project split across three `.kul` files in one flat namespace, with `birth` lines referencing marriages declared in sibling files. |
| [`09-family-across-a-century/`](./09-family-across-a-century/family-across-a-century.kul) | The capstone: a ~30-person dynasty combining every construct above — widowhood, polygamy, divorce and remarriage, adoption, marrying-in from other families, mixed dates, `gender:other`. |

## Layout

Each example is a self-contained Kul project: a numbered subdirectory of `examples/` carrying one or more `.kul` files plus a sibling `kul.yml` manifest. Most examples are single-file; the multi-file example (`08-multi-file-project/`) demonstrates the project-wide namespace landed by [ADR-0015](../docs/adr/0015-global-project-namespace.md) — every `.kul` file in the directory shares one logical namespace, with cross-file `birth` references resolving by bare id.

```
examples/
├── 01-nuclear-family/
│   ├── kul.yml
│   └── nuclear-family.kul
├── 02-three-generations/
│   ├── kul.yml
│   └── three-generations.kul
├── …
└── 08-multi-file-project/
    ├── kul.yml
    ├── 01-founders.kul
    ├── 02-children.kul
    └── 03-grandchildren.kul
```

## Try it

The CLI's `validate`, `format`, and `export` subcommands are CWD-rooted (issue #83) — each operates on the project rooted at the current working directory. From the repo root:

```sh
(cd examples/01-nuclear-family && kul validate)
(cd examples/08-multi-file-project && kul validate)  # multi-file project
```

Every example must exit `0`. Anything that doesn't is either a regression or a new spec rule that hasn't been propagated to the corpus yet.

## Conventions

- Directory names: `NN-short-slug/`, where `NN` orders by complexity (smallest first).
- Single-file examples: the `.kul` file inside the directory drops the `NN-` prefix (e.g. `01-nuclear-family/nuclear-family.kul`).
- Multi-file examples: each `.kul` file is named by the slice of the project it carries, prefixed with `NN-` so alphabetic file order matches reading order (e.g. `01-founders.kul`, `02-children.kul`); the directory itself carries the example number.
- Each `.kul` file starts with a header comment summarizing what it demonstrates.
- Each example is a distinct, internally-coherent family — names don't carry across examples, so each reads on its own.
- Each example is **self-contained** — every referenced ID is declared within that example's project directory (single file or sibling files in the same directory).
- Each example is a **happy path** — examples must validate cleanly. Negative fixtures live next to their tests in `crates/kul-core/tests/`, not here.

For the language reference, see [`spec/`](../spec/README.md). For domain vocabulary, see [`CONTEXT.md`](../CONTEXT.md).
