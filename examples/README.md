# Examples

Worked-example `.kul` documents that double as the **positive test corpus**. Every file here is exercised by integration tests in `crates/kul-core/tests/` and `crates/kul-cli/tests/` — they must always validate cleanly. If you add an example, the test suite will pull it in automatically; if you change one, snapshot tests will flag the diff.

| File                                                    | Demonstrates                                                                              |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| [`01-single-couple.kul`](./01-single-couple.kul)      | Minimal shape: two persons + one ongoing marriage, no children.                           |
| [`02-nuclear-family.kul`](./02-nuclear-family.kul)    | A `birth` sub-statement deriving parents from a marriage.                                 |
| [`03-three-generations.kul`](./03-three-generations.kul) | Three generations, divorce (`end_reason`), retroactive adoption, circa date (`~1980`).  |
| [`04-polygamous-family.kul`](./04-polygamous-family.kul) | Two concurrent marriages for one person; child of one of them.                          |
| [`05-married-siblings.kul`](./05-married-siblings.kul) | Two sons of a couple, each themselves married; one block per marriage.                  |

## Try it

```sh
kul validate examples/*.kul
```

Every example must exit `0`. Anything that doesn't is either a regression or a new spec rule that hasn't been propagated to the corpus yet.

## Conventions

- File names: `NN-short-slug.kul`, where `NN` orders by complexity (smallest first).
- Each file starts with a `# Example N:` header comment summarizing what it demonstrates.
- Each file is **self-contained** — every referenced ID is declared in the same file.
- Each file is a **happy path** — examples must validate cleanly. Negative fixtures live next to their tests in `crates/kul-core/tests/`, not here.

For the language reference, see [`spec/`](../spec/README.md). For domain vocabulary, see [`CONTEXT.md`](../CONTEXT.md).
