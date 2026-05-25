# Multi-file projects

A **Kul project** is one directory containing a `kul.yml` manifest plus one or more `.kul` files. Every id declared in any of those files is visible from every other file by bare name — there is no `import`. Subdirectories are not walked. Normative source: [spec §10 — file conventions](https://github.com/YashBhalodi/kul/blob/main/spec/10-file-conventions.md), [spec §14 — project manifest](https://github.com/YashBhalodi/kul/blob/main/spec/14-project-manifest.md).

## Manifest

```yaml
# kul.yml
kul: "0.1"
```

Required next to the `.kul` files. Missing manifest → `KUL-M01`. Manifest with zero `.kul` siblings → `KUL-M06`.

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
