# 10. File conventions

|                       |                                                                              |
| --------------------- | ---------------------------------------------------------------------------- |
| Source file extension | `.kul`                                                                       |
| Project manifest      | `kul.yml` (one per directory; required — see [Section 14](./14-project-manifest.md)) |
| Encoding              | UTF-8 (no BOM)                                                               |
| Line endings          | LF or CRLF (parser MUST accept either)                                       |
| CLI binary            | `kul` (e.g., `kul validate family.kul`)                                      |

A Kul document MAY be empty (zero statements). Such a document represents the empty family and is valid. The project manifest is still required even for an empty document.

---

← [Section 9 — Edge cases](./09-edge-cases.md) | [Index](./README.md) | Next → [Section 11 — Reserved keywords](./11-reserved-keywords.md)
