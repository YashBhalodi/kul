# Examples

Worked-example `.kula` documents that double as the **positive test corpus**. Every file here is exercised by integration tests in `crates/kula-core/tests/` and `crates/kula-cli/tests/` — they must always validate cleanly. If you add an example, the test suite will pull it in automatically; if you change one, snapshot tests will flag the diff.

| File                                                    | Demonstrates                                                                              |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| [`01-single-couple.kula`](./01-single-couple.kula)      | Minimal shape: two persons + one ongoing marriage, no children.                           |
| [`02-nuclear-family.kula`](./02-nuclear-family.kula)    | A `birth` sub-statement deriving parents from a marriage.                                 |
| [`03-three-generations.kula`](./03-three-generations.kula) | Three generations, divorce (`end_reason`), retroactive adoption, circa date (`~1980`).  |
| [`04-polygamous-family.kula`](./04-polygamous-family.kula) | Two concurrent marriages for one person; child of one of them.                          |
| [`05-married-siblings.kula`](./05-married-siblings.kula) | Two sons of a couple, each themselves married; one block per marriage.                  |

## Try it

```sh
kula validate examples/*.kula
```

Every example must exit `0`. Anything that doesn't is either a regression or a new spec rule that hasn't been propagated to the corpus yet.

## Conventions

- File names: `NN-short-slug.kula`, where `NN` orders by complexity (smallest first).
- Each file starts with a `# Example N:` header comment summarizing what it demonstrates.
- Each file is **self-contained** — every referenced ID is declared in the same file.
- Each file is a **happy path** — examples must validate cleanly. Negative fixtures live next to their tests in `crates/kula-core/tests/`, not here.

For the language reference, see [`spec/`](../spec/README.md). For domain vocabulary, see [`CONTEXT.md`](../CONTEXT.md).
