# Multi-file Kul projects

Reference for splitting large families across multiple `.kul` files inside one project. Sourced from [`spec/10-file-conventions.md`](../../../spec/10-file-conventions.md) and [`spec/14-project-manifest.md`](../../../spec/14-project-manifest.md), with worked patterns from [`examples/07-multi-file-extended-family/`](../../../examples/07-multi-file-extended-family/).

## What a Kul project is

A **Kul project** is a directory containing:

1. One `kul.yml` manifest (required).
2. One or more `.kul` files in the same directory.

Subdirectories are **not walked**. Non-`.kul` files (README, `.gitignore`, editor backups) are silently ignored. The whole project is one logical namespace: every id declared in any `.kul` file is visible from every other file by bare name. There is no `import`, no namespace prefix, no qualified-reference syntax.

A single-file project is the N=1 case of the multi-file shape; the discovery rule is the same.

## The manifest

Every project needs `kul.yml` alongside its `.kul` files:

```yaml
kul: "0.1"
```

One required field — the Kul language version. YAML `#` comments are permitted. Unknown fields are silently ignored (forward-compatibility hedge).

Without `kul.yml`, a directory of `.kul` files is **not a valid Kul project**, and the toolchain will report `KUL-M01` (manifest not found). A `kul.yml` with zero sibling `.kul` files is also invalid (`KUL-M06`).

## When to split files

A heuristic ladder, smallest to largest:

| Family size                          | Recommended shape                                                                                        |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| 1–10 persons                         | One file. Don't split — the cognitive overhead isn't worth it.                                           |
| 10–30 persons                        | One file, sectioned with `# ---- Generation N ----` comment headers. (See `examples/03-three-generations/`.) |
| 30–80 persons, clear branches        | Multiple files, one per generation OR one per family branch. (See `examples/07-multi-file-extended-family/`.) |
| 80+ persons, multiple branches       | Multiple files, one per branch, with a top-level `00-founders.kul` for the root couple.                  |
| Hundreds of persons across centuries | Multi-file by branch, numbered for reading order. Consider further partitioning by generation within a branch. |

The cutoffs are heuristic, not normative. Lean toward one file until the file becomes hard to scan; lean toward more files when sections of the family are independently maintained.

## Partitioning strategies

### By generation

The cleanest split when the family fits naturally into generational tiers. Example 7 in the corpus uses this shape:

```
project/
├── kul.yml
├── 01-founders.kul         # generation 1 + the founding marriage
├── 02-parents.kul          # generation 2 + their marriages
└── 03-grandchildren.kul    # generation 3
```

Pros: easy to read top-down, matches how humans usually narrate a family history.

Cons: a five-generation family ends up with five files; cousins from different branches but the same generation share a file even when they're otherwise unrelated.

### By branch

The cleanest split when the family is a tree with multiple sibling lines that develop independently — e.g. a founder couple has three children and each child's line is itself large.

```
project/
├── kul.yml
├── 00-founders.kul         # founder couple + their marriage
├── 01-branch-arjun.kul     # eldest son's line: him, his spouse, descendants
├── 02-branch-bharat.kul    # second son's line
└── 03-branch-chitra.kul    # daughter's line
```

Pros: each branch is self-contained except for the `birth m_<founders>` references resolving to the founders file.

Cons: a spouse who "marries into" the family lives in their new-family branch; if they bring documented kin (parents, siblings of their own), those may need their own slice.

### Hybrid: branch + generation

For very large families, split by branch first, then within a branch by generation:

```
project/
├── kul.yml
├── 00-founders.kul
├── 10-branch-arjun-gen1.kul
├── 11-branch-arjun-gen2.kul
├── 12-branch-arjun-gen3.kul
├── 20-branch-bharat-gen1.kul
…
```

Number prefixes ensure alphabetic file order matches reading order.

### Anti-pattern: by surname

Don't split by surname. A married-out daughter takes her spouse's surname (often), and her children's surname depends on local convention; you end up scattering one branch across multiple files. Split by branch (lineage) or by generation — both track the *graph* rather than the *labels*.

## Cross-file references

There are no special markers for cross-file references. An id declared in `01-founders.kul` can be referenced from `02-parents.kul` exactly the same way it would be referenced within `01-founders.kul`:

```
# in 01-founders.kul
marriage m_ramesh_sita ramesh sita  start:1952-02-18

# in 02-parents.kul
person alice  name:"Alice Patel"  gender:female  born:1955-07-19
  birth m_ramesh_sita        # resolves across files; no import needed
```

Validation, including duplicate-id detection (KUL-R01) and parenthood-cycle detection (KUL-R13), is project-wide. Cross-file duplicates fire as a single duplicate-id error; cross-file cycles fire as a single cycle.

## File naming conventions

Conventions from the example corpus:

- **Single-file project** — the `.kul` file's basename matches the project's purpose (`single-couple.kul`, `nuclear-family.kul`). It can drop any leading `NN-` prefix.
- **Multi-file project** — each file is prefixed with a two-digit number (`01-founders.kul`, `02-parents.kul`) so alphabetic order matches reading order.
- **Encoding** — UTF-8, no BOM. Line endings LF or CRLF.

## Header comments

Each `.kul` file should start with a brief `#` header summarizing what it carries — which generation, which branch, what role it plays in the whole. Example:

```
# Example 7 (file 2 of 3) — Second generation.
#
# Each child of `m_ramesh_sita` declares their bio-link via `birth
# m_ramesh_sita` — that marriage id is declared in `01-founders.kul`,
# in this same project. Spouses for the second-generation marriages
# are also declared here, alongside their marriages.
```

This is the human's map. The toolchain doesn't care, but a reader scanning the directory does.

## Empty / placeholder files

A project may have one `.kul` file that is empty (zero statements). That's a valid empty family. But a project must have *at least one* `.kul` file — `kul.yml` alone is `KUL-M06`.

## Practical authoring loop

When authoring a fresh project from prose:

1. **Get the manifest in place first** — `kul.yml` with `kul: "0.1"`.
2. **Decide on file partitioning** before writing any declarations. It's much easier to start with the right shape than to split later. (Splitting is mechanically easy — declarations are file-position-independent — but it's still effort.)
3. **Write the founders' file first.** Founders are the easiest declarations (often no `birth` sub-statement, dates often partial / circa). Establishing the founder ids early gives the rest of the project something to point at.
4. **Write each branch / generation file** in turn, with `birth` / `adoption` references pointing back at earlier files.
5. **Header-comment every file** with what slice it carries and which sibling files it references.
