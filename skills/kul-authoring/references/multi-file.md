# Multi-file projects

A **Kul project** is the `.kul` files directly inside one directory — no other file is needed. Every id declared in any of those files is visible from every other file by bare name — there is no `import`. Subdirectories are not walked, and files with other extensions are ignored. Normative source: [spec §2 — projects](https://github.com/YashBhalodi/kul/blob/main/spec/02-document-structure.md#projects), [spec §10 — file conventions](https://github.com/YashBhalodi/kul/blob/main/spec/10-file-conventions.md).

## File order and naming

Files are read in byte-wise order of their names (`10-x.kul` sorts before `9-x.kul`, and uppercase before lowercase). References resolve in every direction, so order never decides validity; it decides **declaration order** — the order an export lists entities in, and which of two duplicate ids is reported. Name files with a zero-padded prefix (`01-founders.kul`, `02-parents.kul`) so file order matches reading order.

## Cross-file references

Reference any id by bare name; the whole directory is one logical namespace.

```
# 01-founders.kul
marriage m_ramesh_sita ramesh sita  start:1952-02-18
```
```
# 02-parents.kul
person alice  name:"Alice Patel"  gender:female  born:1955-07-19
  birth m_ramesh_sita        # resolves across files
```

Validation is project-wide: duplicate ids across files fire as one `KUL-R01`; parenthood cycles across files fire as one `KUL-R13`. Whether and how to split a project across files is the author's choice — see [examples/08-multi-file-project](https://github.com/YashBhalodi/kul/tree/main/examples/08-multi-file-project) for one worked split.
