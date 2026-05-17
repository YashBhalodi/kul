# Multi-file projects

A **Kul project** is one directory containing a `kul.yml` manifest plus one or more `.kul` files. Every id declared in any of those files is visible from every other file by bare name — there is no `import`. Subdirectories are not walked. Normative source: [`spec/10`](../../../spec/10-file-conventions.md), [`spec/14`](../../../spec/14-project-manifest.md).

## Manifest

```yaml
# kul.yml
kul: "0.1"
```

Required next to the `.kul` files. Missing manifest → `KUL-M01`. Manifest with zero `.kul` siblings → `KUL-M06`.

## When to split

Heuristic, not normative: split when a single file becomes hard to scan top-down. Roughly ~30 persons is where partitioning starts earning its keep. The two clean shapes:

- **By generation** — `01-founders.kul`, `02-parents.kul`, `03-grandchildren.kul`. Matches narrative order. Used in [`examples/07-multi-file-extended-family/`](../../../examples/07-multi-file-extended-family/).
- **By branch** — `00-founders.kul`, `01-branch-arjun.kul`, `02-branch-bharat.kul`. Use when each branch is large and develops independently.

Don't split by surname — a child whose surname comes from their other parent scatters across files. Split by the graph (generation or branch), not by the labels.

## Cross-file references

Just reference by id; the whole directory is one logical namespace.

```
# 01-founders.kul
marriage m_ramesh_sita ramesh sita  start:1952-02-18
```
```
# 02-parents.kul
person alice  name:"Alice Patel"  gender:female  born:1955-07-19
  birth m_ramesh_sita        # resolves across files — no import needed
```

Validation is project-wide: duplicate ids across files fire as one `KUL-R01`; parenthood cycles across files fire as one `KUL-R13`.
