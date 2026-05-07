# 10. File conventions

|                |                                            |
| -------------- | ------------------------------------------ |
| File extension | `.kul`                                    |
| Encoding       | UTF-8 (no BOM)                             |
| Line endings   | LF or CRLF (parser MUST accept either)     |
| CLI binary     | `kul` (e.g., `kul validate family.kul`) |

A Kul document MAY be empty (zero statements). Such a document represents the empty family and is valid.

---

← [Section 9 — Edge cases](./09-edge-cases.md) | [Index](./README.md) | Next → [Section 11 — Reserved keywords](./11-reserved-keywords.md)
