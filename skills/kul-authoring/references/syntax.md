# Syntax

Condensed reference for Kul 0.1. Normative source: [spec sections 3–6](https://github.com/YashBhalodi/kul/tree/main/spec) — lexical structure, top-level statements, sub-statements, semantics — plus [reserved keywords](https://github.com/YashBhalodi/kul/blob/main/spec/11-reserved-keywords.md), [formal grammar](https://github.com/YashBhalodi/kul/blob/main/spec/12-formal-grammar.md), and the standalone [grammar.ebnf](https://github.com/YashBhalodi/kul/blob/main/spec/grammar.ebnf). One micro-example per construct.

## Document shape

UTF-8, no BOM, LF or CRLF. One statement per line. Top-level: `person`, `marriage`. Sub-statements (`birth`, `adoption`) are indented continuations of the most recent `person`. Statement order is free — forward references resolve cleanly. The language version lives in `kul.yml` (`kul: "0.1"`), not inside `.kul`.

```
# 01-family.kul
person alice  name:"Alice"  gender:female
person bob    name:"Bob"    gender:male

marriage m_alice_bob alice bob  start:1972
```

## `person`

```
person alice  name:"Alice Sharma"  gender:female  family:"Sharma"  given:"Alice"  born:1950-04-12
```

| Field    | Required | Type   | Notes                                       |
| -------- | -------- | ------ | ------------------------------------------- |
| `name`   | **yes**  | string | Display name; full UTF-8.                   |
| `gender` | **yes**  | enum   | `male` \| `female` \| `other`.              |
| `family` | no       | string | Family name.                                |
| `given`  | no       | string | Given name.                                 |
| `born`   | no       | date   | Date of birth.                              |
| `died`   | no       | date   | Date of death. Absence means alive.         |

## `marriage`

```
marriage m_alice_bob alice bob  start:1972-05-12  end:1990-08-01  end_reason:divorce
```

| Field        | Required        | Type | Notes                                                                          |
| ------------ | --------------- | ---- | ------------------------------------------------------------------------------ |
| `start`      | **yes**         | date | Date marriage began.                                                           |
| `end`        | no              | date | Date marriage ended. Absence means ongoing or ended only by spousal death.     |
| `end_reason` | iff `end` given | enum | Required iff `end` is present. v1 vocabulary: `divorce`.                       |

Spouse positions must be distinct. Spouse order has no semantic meaning. Spousal death does not auto-end a marriage (see [spec §6.2 — active marriage at time T](https://github.com/YashBhalodi/kul/blob/main/spec/06-semantics.md#62-active-marriage-at-time-t)).

## `birth` (sub-statement)

```
person carol  name:"Carol"  gender:female  born:1975-09-03
  birth m_alice_bob
```

At most one per `person`. Two-space indent (canonical formatter rule; spec allows any). The bio-birth date is the person's `born:` field — `birth` carries no fields.

## `adoption` (sub-statement)

```
person ravi  name:"Ravi"  gender:male  born:~1980
  adoption m_alice_bob  start:1985-06-01
```

Zero or more per `person`. Permanent unless `end:<date>` is given. No `end_reason` field. A person may have both `birth` and one or more `adoption`s.

## Date literals

`YYYY` | `YYYY-MM` | `YYYY-MM-DD`, optionally prefixed `~` for circa (±5y tolerance in validation). Written bare. Match the granularity of the prose — no `unknown` literal exists; omit the field instead.

```
born:1925           # exact year
born:1925-03-10     # exact full date
born:~1925          # approximately 1925
```

## Strings, bare values, comments

- Strings: double-quoted UTF-8. Escapes: `\"`, `\\`. Quote anything containing whitespace, `:`, `#`, or `"`.
- Bare values: single tokens with no whitespace/`:`/`"`/`#`. Conventional for enums (`gender:female`, `end_reason:divorce`) and id references.
- Comments: `#` to end-of-line. Preserved verbatim by the formatter.

## Identifiers

`[A-Za-z_][A-Za-z0-9_-]*`, case-sensitive. Must not match one of the 17 reserved keywords ([spec §11](https://github.com/YashBhalodi/kul/blob/main/spec/11-reserved-keywords.md)):

```
adoption  birth   born  died    divorce  end       end_reason  family  female
gender    given   male  marriage  name    other     person      start
```
