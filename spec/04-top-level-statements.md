# 4. Top-level statements

## 4.1 Version declaration

The first non-blank, non-comment line of a document MAY be a version declaration:

```
kula 0.1
```

The token following `kula` is a version number in `MAJOR.MINOR` form. A parser that does not recognize the version SHOULD report an error rather than parsing the document. If the version declaration is absent, the version is assumed to be the latest version known to the parser.

A document MUST NOT contain more than one version declaration.

## 4.2 Person statement

A person statement declares a person and their fields, optionally followed by indented sub-statements:

```
person <id> <field>...
  <sub-statement>...
```

Where `<id>` is an identifier unique within the document. `<id>` is positional and required. Fields and their semantics:

| Field    | Required | Type   | Notes                                   |
| -------- | -------- | ------ | --------------------------------------- |
| `name`   | yes      | string | Display name; full UTF-8                |
| `gender` | yes      | enum   | One of `male`, `female`, `other`        |
| `family` | no       | string | Family name (e.g., for derived queries) |
| `given`  | no       | string | Given name (e.g., for derived queries)  |
| `born`   | no       | date   | Date of birth                           |
| `died`   | no       | date   | Date of death; absence means alive      |

Sub-statements `birth` and `adoption` are described in [Section 5 — Person sub-statements](./05-person-sub-statements.md).

Example:

```
person alice name:"Alice Sharma" family:"Sharma" given:"Alice" born:1950-04-12 gender:female
  birth m_ramesh_sita
```

## 4.3 Marriage statement

A marriage statement declares a marriage between two persons:

```
marriage <id> <spouse-a> <spouse-b> <field>...
```

Where `<id>` is an identifier unique within the document, and `<spouse-a>` and `<spouse-b>` are identifiers referring to declared persons. The `<id>` and the two spouse identifiers are positional and required. The order of the two spouse identifiers carries no semantic significance.

Fields:

| Field        | Required    | Type | Notes                                                                                        |
| ------------ | ----------- | ---- | -------------------------------------------------------------------------------------------- |
| `start`      | yes         | date | Date the marriage began                                                                      |
| `end`        | no          | date | Date the marriage ended (e.g. divorce); absence means ongoing or ended only by spousal death |
| `end_reason` | conditional | enum | Required iff `end` is present; v1 vocabulary: `divorce`                                      |

A spousal death does NOT auto-end a marriage. See [Section 6.2 — Active marriage at time T](./06-semantics.md#62-active-marriage-at-time-t).

A marriage's two spouse identifiers MUST be distinct (a person cannot marry themselves; see [Section 7 — Validation rules](./07-validation-rules.md), rule 4).

Example:

```
marriage m_alice_bob alice bob start:1972-05-12 end:1990-08-01 end_reason:divorce
marriage m_alice_devraj alice devraj start:1992-02-14
```

---

← [Section 3 — Lexical structure](./03-lexical-structure.md) | [Index](./README.md) | Next → [Section 5 — Person sub-statements](./05-person-sub-statements.md)
