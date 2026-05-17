# Syntax digest

Condensed reference for Kul 0.1 grammar, sourced from [`spec/03-lexical-structure.md`](../../../spec/03-lexical-structure.md) through [`spec/06-semantics.md`](../../../spec/06-semantics.md), [`spec/11-reserved-keywords.md`](../../../spec/11-reserved-keywords.md), and [`spec/12-formal-grammar.md`](../../../spec/12-formal-grammar.md) (with the full EBNF in [`spec/grammar.ebnf`](../../../spec/grammar.ebnf)). One micro-example per construct.

If this file disagrees with the spec, the spec wins.

## Document shape

A Kul **document** is a sequence of statements, one per line. Top-level lines are `person` or `marriage`; sub-statements (`birth`, `adoption`) are indented continuations of the most recent `person`. The Kul language version lives in the sibling `kul.yml` manifest, not in the `.kul` source. Encoding is UTF-8 (no BOM); line endings LF or CRLF.

A document MAY be empty. Statement order is free — forward references are permitted. Reference resolution happens after the full document is parsed.

```
kul: "0.1"        # this lives in kul.yml, not in the .kul file
```

```
# a .kul file
person alice  name:"Alice"  gender:female
person bob    name:"Bob"    gender:male

marriage m_alice_bob alice bob  start:1972
```

## Comments

`#` starts a comment that runs to end of line. Both whole-line and trailing comments are supported. Block comments do not exist.

```
# whole-line comment
person alice name:"Alice" gender:female  # trailing comment
```

## Identifiers

`[A-Za-z_][A-Za-z0-9_-]*`. Case-sensitive. **Must not** be one of the 17 reserved keywords below.

```
person ramesh_sr name:"Ramesh Sharma Sr." gender:male
```

### Reserved keywords (17, exhaustive for v1)

```
adoption    birth       born        died        divorce
end         end_reason  family      female      gender
given       male        marriage    name        other
person      start
```

You cannot use any of these as a person or marriage id. Pick `marriage_2` instead of `marriage`, `birth_record` instead of `birth`, etc.

## String literals

Double-quoted UTF-8. Two escapes: `\"` and `\\`. No others. Strings may contain spaces, colons, hashes, and quotes (the latter escaped).

```
person foo name:"O'Brien \"Slim\" McGee" gender:male
```

## Bare values

When a value is a single token with no whitespace, no `:`, no `"`, and no `#`, you may write it bare. Conventional for enums and identifier references.

```
gender:female              # bare (equivalent to "female")
end_reason:divorce         # bare (the only v1 enum value for end_reason)
birth m_alice_bob          # bare identifier reference
name:"Alice Sharma"        # must be quoted (contains a space)
```

## Date literals

Three granularities, optional `~` prefix for circa (±5-year tolerance during validation). Written bare (no quotes).

| Form         | Example         | Means                                |
| ------------ | --------------- | ------------------------------------ |
| `YYYY`       | `1980`          | Exact year                           |
| `YYYY-MM`    | `1980-03`       | Exact year and month                 |
| `YYYY-MM-DD` | `1980-03-15`    | Exact full date                      |
| `~YYYY[…]`   | `~1980-03`      | Approximate, with ±5-year tolerance  |

Month is `01..12`; day must be valid for the month (incl. leap years). There is no syntax for "unknown" — absence of a date field expresses "not recorded." Coarser granularity expresses "known partially." Circa expresses "approximate."

```
born:1925           # exact year
born:1925-03        # exact year-month
born:1925-03-10     # exact full date
born:~1925          # approximately 1925
```

## Field syntax

`<name>:<value>` with **no whitespace** between the name, `:`, and value. Field order within a statement is free for the parser; the formatter canonicalizes it (see [`spec/15-formatter-rules.md`](../../../spec/15-formatter-rules.md)).

```
name:"Alice Sharma"
born:1950-04-12
gender:female
end_reason:divorce
```

A field repeated within the same declaration is an error (rule KUL-R05 — caught by the validator, not the parser).

## `person` statement

```
person <id> <field>...
  <sub-statement>...
```

Fields:

| Field    | Required | Type    | Notes                                      |
| -------- | -------- | ------- | ------------------------------------------ |
| `name`   | **yes**  | string  | Display name; full UTF-8.                  |
| `gender` | **yes**  | enum    | `male` \| `female` \| `other`.             |
| `family` | no       | string  | Family name (for derived queries).         |
| `given`  | no       | string  | Given name (for derived queries).          |
| `born`   | no       | date    | Date of birth.                             |
| `died`   | no       | date    | Date of death. Absence means alive.        |

```
person alice  name:"Alice Sharma"  gender:female  family:"Sharma"  given:"Alice"  born:1950-04-12
```

A `person` may carry sub-statements (`birth`, `adoption`) on the lines immediately following, each indented.

## `marriage` statement

```
marriage <id> <spouse-a> <spouse-b> <field>...
```

`<id>`, `<spouse-a>`, `<spouse-b>` are positional and required. The two spouse identifiers must be distinct (rule KUL-R04). Spouse order is not semantic.

Fields:

| Field        | Required        | Type    | Notes                                                                       |
| ------------ | --------------- | ------- | --------------------------------------------------------------------------- |
| `start`      | **yes**         | date    | Date the marriage began.                                                    |
| `end`        | no              | date    | Date the marriage ended. Absence means ongoing or ended only by spousal death. |
| `end_reason` | iff `end` given | enum    | Required iff `end` is present. v1 vocabulary: `divorce`.                    |

```
marriage m_alice_bob alice bob  start:1972-05-12  end:1990-08-01  end_reason:divorce
marriage m_alice_devraj alice devraj  start:1992-02-14
```

Spousal death does NOT auto-end a marriage; the marriage's record is unchanged. Spec §6.2 — "active marriage at time T" — derives the practical "currently married" answer from the spouses' `died:` fields.

## `birth` sub-statement

```
  birth <marriage-id>
```

Indented under a `person`. Declares this person was biologically born to the union represented by `<marriage-id>`. The two spouses of that marriage are this person's biological parents.

A person MUST have at most one `birth` sub-statement. Absence is permitted and means "biological parents undocumented" — the person is an implicit documentation root.

```
person carol  name:"Carol Sharma"  gender:female  born:1975-09-03
  birth m_alice_bob
```

The date of biological birth is the person's `born:` field; no separate date on `birth`.

## `adoption` sub-statement

```
  adoption <marriage-id> start:<date> [end:<date>]
```

Indented under a `person`. Declares this person was adopted into `<marriage-id>` on `start:<date>`. Adoptions are permanent unless `end:<date>` is given. A person MAY have zero or more `adoption` sub-statements; there is no `end_reason` on adoption in v1.

```
person ravi  name:"Ravi Sharma"  gender:male  born:~1980
  adoption m_alice_bob  start:1985-06-01
```

A person may have both a `birth` (their biological origin) and one or more `adoption`s; all surface in the derived parent set. Multiple adoptions, one ended:

```
person someone  name:"Someone"  gender:female  born:1985-01-01
  adoption m_first_couple   start:1985-06-01  end:1990-01-01
  adoption m_second_couple  start:1992-04-15
```

## Indentation

Sub-statements MUST be indented with exactly two ASCII spaces (formatter canonical; the lexer accepts any whitespace amount but tabs are a parse error). One or more spaces is acceptable to the parser, but the formatter normalizes to two.

```
person alice name:"Alice" gender:female
  birth m_parents       # exactly two spaces under canonical form
```

A line beginning in column 1 is a top-level statement. A line beginning with horizontal whitespace is a sub-statement of the most recent `person`. Blank lines are permitted anywhere and never bind a sub-statement.

## Forward references and resolution

Statements may appear in any order. The parser collects all top-level `person` and `marriage` declarations first, then resolves references in a second phase. Spouse identifiers, `birth` marriage-ids, and `adoption` marriage-ids all resolve project-wide (across every `.kul` file in the same `kul.yml` project — see [`multi-file.md`](./multi-file.md) for cross-file resolution).

```
# declaration order is free
person carol name:"Carol" gender:female
  birth m_alice_bob              # forward reference — fine

marriage m_alice_bob alice bob  start:1972     # resolved after the parse
person alice name:"Alice" gender:female        # also resolved after the parse
person bob   name:"Bob"   gender:male
```

## Things the grammar does NOT enforce (the validator does)

The 13 spec-defined rules (`KUL-R01` … `KUL-R13`) cover:

- Duplicate ids across the whole project.
- Unresolved references (a `birth`/`adoption`/spouse id that doesn't match any declaration).
- Required fields missing (`name`, `gender` on `person`; both spouses + `start` on `marriage`).
- Self-marriage (`marriage m alice alice` — both spouse positions must differ).
- End consistency (`end` and `end_reason` must both be present or both absent).
- Eight temporal-impossibility checks (died-before-born, marriage-before-spouse-born, child-born-before-parent, adoption-before-adopter-born, etc.).
- Parenthood cycles (no person may appear as their own ancestor through any combination of `birth`/`adoption` links).

You should aim to author source that passes all of these, but the skill does not enumerate rule logic. See [`spec/07-validation-rules.md`](../../../spec/07-validation-rules.md) when a rule's exact text becomes load-bearing for an authoring decision.

## Things that look possible but aren't

- **No `parent` field on `person`.** Parents are reached through `birth`/`adoption` sub-statements pointing at marriages.
- **No `child` keyword.** Children are reached through their own `birth`/`adoption` sub-statements.
- **No `sibling` declaration.** Siblings are derived.
- **No `import` / `include`.** Multi-file projects share one namespace (see [`multi-file.md`](./multi-file.md)).
- **No `kul 0.1` line inside `.kul` files.** The language version lives in `kul.yml`.
- **No friendship, engagement, or non-marital partnership statements.** v1 is kinship-only.
- **No `end_reason:death`.** Spousal death does not end a marriage in the record. The v1 enum value for `end_reason` is `divorce` only.
- **No date literal for "unknown."** Omit the field; the absence is the canonical "not recorded" signal.
- **No `~` on anything other than dates.** Circa is a date-only modifier.
