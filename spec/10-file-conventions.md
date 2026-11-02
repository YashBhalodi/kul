# 10. File conventions

|                       |                                                                              |
| --------------------- | ---------------------------------------------------------------------------- |
| Source file extension | `.kul`                                                                       |
| Project               | Every `.kul` file directly inside one directory — see [Section 2](./02-document-structure.md#projects) |
| Encoding              | UTF-8 (no BOM)                                                               |
| Line endings          | LF or CRLF (parser MUST accept either)                                       |
| CLI binary            | `kul` (`kul validate [DIR]`; `DIR` defaults to the current directory)        |

A Kul document MAY be empty (zero statements). Such a document represents the empty family and is valid; so is a project with no documents.

---

← [Section 9 — Edge cases](./09-edge-cases.md) | [Index](./README.md) | Next → [Section 11 — Reserved keywords](./11-reserved-keywords.md)
