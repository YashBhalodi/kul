# 10. File conventions

|                |                                            |
| -------------- | ------------------------------------------ |
| File extension | `.kula`                                    |
| Encoding       | UTF-8 (no BOM)                             |
| Line endings   | LF or CRLF (parser MUST accept either)     |
| CLI binary     | `kula` (e.g., `kula validate family.kula`) |

A Kula document MAY be empty (zero statements). Such a document represents the empty family and is valid.

---

← [Section 9 — Edge cases](./09-edge-cases.md) | [Index](./README.md) | Next → [Section 11 — Reserved keywords](./11-reserved-keywords.md)
