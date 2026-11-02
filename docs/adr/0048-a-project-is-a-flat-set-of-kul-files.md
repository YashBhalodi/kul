# ADR 0048 — A project is a flat set of `.kul` files, with no project file

**Status:** Accepted
**Date:** 2026-11-02
**Deciders:** owner

Supersedes [ADR-0013](./0013-project-manifest.md), [ADR-0014](./0014-file-identity-and-per-file-namespaces.md) (restating the file-identity types it introduced) and [ADR-0030](./0030-replace-serde-yaml-with-serde-norway.md). Amends [ADR-0015](./0015-global-project-namespace.md) (what a project is and how it is found). Issue [#345](https://github.com/YashBhalodi/kul/issues/345).

## Context

A project used to be a directory holding a YAML project file next to its `.kul` files. The file declared the language version and nothing else; it brought a YAML parser into `kul-core`, a family of project-file diagnostics, a crate whose only job was to find the file and its siblings, and a reserved `FileId(0)` that every caller had to offset around.

The browser has no directory: the app's project store holds a set of named files (ADR-0051). A project definition that needs a file on disk does not fit it, and a version declaration that only says "0.1" earns none of that cost. The language version a document is written against is the specification's; a tool reports the version it implements.

## Decision

### A project is its files

A **project** is a flat set of uniquely named `.kul` files. Read from a directory, it is every file directly inside it whose name ends in `.kul`; subdirectories are not walked and other files are ignored. A project with no files, or only empty ones, is the empty family and is valid. Spec §2, *Projects*, is normative. Nothing in a project declares a language version.

### File order is owned by `kul_core::check`

`kul_core::check(&[InputFile]) -> CheckResult` puts its inputs in **file order** itself: the byte-wise lexicographic order of their names, comparing UTF-8 bytes (`B.kul` before `a.kul`, `10.kul` before `9.kul`). Declaration order — which duplicate id is "the second" for KUL-R01, the order of the export's collections — is file order, then source position.

Core owns the rule so that every caller gets it for free and they cannot disagree: the CLI lists a directory in whatever order the OS returns, the browser's `Workspace` holds files in a map, and both hand the set to `check`. JavaScript's string order compares UTF-16 code units, which differs from UTF-8 byte order for characters outside the Basic Multilingual Plane, so the browser never sorts for the engine.

Core does not diagnose duplicate file names; uniqueness is the caller's guarantee (a directory gives it, and the project store enforces it, case-insensitively). If two inputs share a name anyway, the sort is stable.

### File identity

The types [ADR-0014](./0014-file-identity-and-per-file-namespaces.md) introduced stay, without the reserved slot:

- `InputFile { name, source }` is one file handed to `check`.
- `FileId(i)` is the `i`-th file in file order. There is no reserved id.
- `FileSpan { file, span }` is the project-wide locator a diagnostic anchors on; AST nodes keep a bare `ByteSpan`, their owning `KulFile` giving the file.
- `Diagnostic.primary` stays `Option<FileSpan>`: a few diagnostics describe the request or the render rather than a declaration (an unknown query anchor, KUL-V01).

### The CLI takes `[DIR]`

Every subcommand takes the project directory as its last positional argument, defaulting to `.`: `kul validate [DIR]`, `kul format [DIR]`, `kul export [DIR]`, `kul query person <id> [DIR]`, `kul query kin <anchor> <relation> [DIR]`, `kul query rel <x> <y> [DIR]`, `kul query persons [DIR]`. A missing or unreadable directory exits 1 with `kul: failed to read project directory <dir>: …`. Diagnostics name files by their bare file name whatever `[DIR]` was, so output reads the same wherever the command runs.

### The envelope's `kul` field is `LANGUAGE_VERSION`

The export envelope keeps its `kul` field, which now reports `kul_core::export::LANGUAGE_VERSION` — the version of the specification this toolchain implements — rather than a version read from a project. Its value is unchanged (`"0.1"`), so every export is byte-identical.

### What goes

The YAML dependency and the `yaml` feature, the project-file module and its diagnostics, `check_with_manifest`-style entry points, the directory-walking crate, the unanchored-error constructor only the missing-file diagnostic used, and the project-file fixtures under `examples/` and `crates/kul-core/tests/`.

## Consequences

- The spec's §14 (the project file) is gone; the formatter and export sections are renumbered §14 and §15.
- Every export, SVG, CLI output and language-feature answer over `examples/` is unchanged; the epic's baseline diff proves it.
- The project store and the CLI share one membership rule (`*.kul`, flat) and one order, by construction.
- A future language version is reported by tools, not declared by documents. If a document ever needs to declare one, that is a language change (an in-file declaration), not a project file.

## Anti-suggestions (do not re-propose)

- **"Sort the files in each caller."** Two sorts drift; the browser's natural sort is the wrong one.
- **"Walk subdirectories."** A project is one namespace; nested folders invite per-folder scoping that the language does not have.
- **"Bring back a project file for settings."** UI state belongs to the app's store (ADR-0051), and language facts belong in the language.
