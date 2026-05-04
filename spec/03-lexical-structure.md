# 3. Lexical structure

## 3.1 Comments

A `#` character begins a comment that extends to the end of the line. Comments may appear on their own line or after content on a line. Block comments are not supported in v1.

```
# This is a comment.
person alice name:"Alice"  # comment after content
```

## 3.2 Identifiers

An identifier is `[A-Za-z_][A-Za-z0-9_-]*` — must begin with a letter or underscore; may continue with letters, digits, underscores, and hyphens. Identifiers are case-sensitive; `alice` and `Alice` are different identifiers.

Identifiers are used as person and marriage IDs. They MUST NOT be any of the reserved keywords listed in [Section 11 — Reserved keywords](./11-reserved-keywords.md).

## 3.3 String literals

A string literal is a sequence of characters enclosed in double quotes: `"Alice Sharma"`. Two escape sequences are recognized:

| Escape | Meaning              |
| ------ | -------------------- |
| `\"`   | Literal double quote |
| `\\`   | Literal backslash    |

No other escape sequences are recognized; a backslash followed by any other character is a lexical error. String contents may contain any valid UTF-8 character including newlines, but newlines inside a string are unconventional and tools may discourage them.

## 3.4 Bare values

Where a value is a single token containing no whitespace, no `:`, no `"`, and no `#`, it MAY be written without quotes. The following are equivalent:

```
gender:female
gender:"female"
```

Use of bare values is conventional for enumerations (`gender:male`, `end_reason:divorce`) and identifier references (`birth m_alice_bob`). Strings that contain a space, colon, hash, or quote MUST be quoted.

## 3.5 Date literals

A date literal denotes a date with one of three granularities:

| Granularity | Form         | Example      |
| ----------- | ------------ | ------------ |
| Full date   | `YYYY-MM-DD` | `1975-09-03` |
| Year-month  | `YYYY-MM`    | `1975-09`    |
| Year only   | `YYYY`       | `1975`       |

Year is exactly 4 digits. Month, when present, is exactly 2 digits in `01..12`. Day, when present, is exactly 2 digits and must be valid for the given month and year (`02-30` is invalid; `02-29` is valid only in leap years).

A date literal MAY be prefixed with `~` to indicate **circa** — the date is approximate, with imprecision exceeding the literal granularity:

```
born:1925           # exact year, day unknown
born:~1925          # approximately 1925
born:~1925-03       # approximately March 1925
born:~1925-03-15    # approximately the 15th of March 1925
```

Date literals are written bare (without quotes). The `~` is the only modifier; no other date markers are supported.

There is no syntax for unknown dates. Absence of a date field expresses "not recorded." Coarser granularity expresses "known partially." Circa expresses "approximate." These three mechanisms are sufficient.

## 3.6 Field syntax

A field is `<name>:<value>` written as a single token group with no whitespace between the name, the colon, and the value. The name is one of the field-name keywords listed in [Section 11 — Reserved keywords](./11-reserved-keywords.md). The value is a string literal, bare value, date literal, or identifier as appropriate to the field.

```
name:"Alice Sharma"
born:1950-04-12
gender:female
end_reason:divorce
```

Field order within a statement is free.

---

← [Section 2 — Document structure](./02-document-structure.md) | [Index](./README.md) | Next → [Section 4 — Top-level statements](./04-top-level-statements.md)
