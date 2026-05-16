# Examples

Worked-example `.kul` projects that double as the **positive test corpus**. Each example is its own per-directory Kul project (one `.kul` file plus a sibling `kul.yml` manifest), and every example is exercised by integration tests in `crates/kul-core/tests/` and `crates/kul-cli/tests/` — they must always validate cleanly. If you add an example, the test suite will pull it in automatically; if you change one, snapshot tests will flag the diff.

| Project                                                                                                  | Demonstrates                                                                                                   |
| -------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| [`01-single-couple/single-couple.kul`](./01-single-couple/single-couple.kul)                             | Minimal shape: two persons + one ongoing marriage, no children.                                                |
| [`02-nuclear-family/nuclear-family.kul`](./02-nuclear-family/nuclear-family.kul)                         | A `birth` sub-statement deriving parents from a marriage.                                                      |
| [`03-three-generations/three-generations.kul`](./03-three-generations/three-generations.kul)             | Three generations, divorce (`end_reason`), retroactive adoption, circa date (`~1980`).                         |
| [`04-polygamous-family/polygamous-family.kul`](./04-polygamous-family/polygamous-family.kul)             | Two concurrent marriages for one person; child of one of them.                                                 |
| [`05-married-siblings/married-siblings.kul`](./05-married-siblings/married-siblings.kul)                 | Two sons of a couple, each themselves married; one block per marriage.                                         |
| [`06-three-branch-dynasty/three-branch-dynasty.kul`](./06-three-branch-dynasty/three-branch-dynasty.kul) | Three-branch dynasty: founders, three married children, four married grandchildren spread across the branches. |

## Layout

Each example is a self-contained Kul project: a numbered subdirectory of `examples/` carrying exactly one `.kul` file and a sibling `kul.yml` manifest. Per [ADR-0014](../docs/adr/0014-file-identity-and-per-file-namespaces.md) every example continues to validate within its own per-file scope.

```
examples/
├── 01-single-couple/
│   ├── kul.yml
│   └── single-couple.kul
├── 02-nuclear-family/
│   ├── kul.yml
│   └── nuclear-family.kul
└── …
```

## Try it

```sh
kul validate examples/01-single-couple/single-couple.kul
```

Every example must exit `0`. Anything that doesn't is either a regression or a new spec rule that hasn't been propagated to the corpus yet.

## Conventions

- Directory names: `NN-short-slug/`, where `NN` orders by complexity (smallest first).
- The `.kul` file inside the directory drops the `NN-` prefix (e.g. `01-single-couple/single-couple.kul`).
- Each `.kul` file starts with a `# Example N:` header comment summarizing what it demonstrates.
- Each example is **self-contained** — every referenced ID is declared in that example's own `.kul` file.
- Each example is a **happy path** — examples must validate cleanly. Negative fixtures live next to their tests in `crates/kul-core/tests/`, not here.

For the language reference, see [`spec/`](../spec/README.md). For domain vocabulary, see [`CONTEXT.md`](../CONTEXT.md).
