# Kula Language Specification

**Version 0.1**

> Kula — a kinship description language.

This document specifies the Kula language, version 0.1. It is a normative specification: a conforming parser, validator, or other tool for Kula must implement the rules in this document. It is intentionally rigorous enough that an independent implementation can be written from this document alone.

For background on what Kula is and why it exists, see [VISION.md](./VISION.md).

---

## 1. Introduction

Kula is a domain-specific language for describing human kinship — the structure of families and how they evolve over time. A Kula document is a plain UTF-8 text file with the `.kula` extension. The contents describe persons and the marriages between them; biological parenthood and adoption are recorded as references inside person declarations.

The two primitives of Kula are:

- **Person** — an identifiable individual.
- **Marriage** — a temporal binary union between two persons.

Parenthood (biological and adoptive) is _not_ a separate primitive. It is represented as references on a Person, pointing to the Marriage that produced them (biological birth) or admitted them as a child (adoption).

Out of scope for v1: non-marriage romantic partnerships, sperm donors and surrogates, single parenthood, polyamorous co-parenting (more than two parents in the same parenthood unit), friendships, professional relationships, location and biographical data, multi-file documents, cultural prohibitions on marriage. See VISION.md for the full scope statement.

---

## 2. Document structure

A Kula document is a sequence of **statements**, each starting on its own line. The first non-blank, non-comment line of a document MAY be a version declaration. All other top-level lines are person or marriage statements. Statement order is free; the parser performs reference resolution across the whole document.

The valid top-level statement keywords are:

| Keyword    | Meaning                                                                |
| ---------- | ---------------------------------------------------------------------- |
| `kula`     | Document version declaration (optional; if present, must appear first) |
| `person`   | Person declaration                                                     |
| `marriage` | Marriage declaration                                                   |

Sub-statements (`birth`, `adoption`) appear as indented continuation lines following a `person` statement; they are not valid as top-level statements.

A document MAY contain zero statements; this represents the empty family and is valid.

Whitespace conventions:

- **Within a line:** any amount of horizontal whitespace (spaces and tabs) separates tokens. Trailing whitespace is ignored.
- **Between lines:** blank lines are permitted anywhere and are ignored.
- **Indentation:** a line beginning with horizontal whitespace is a **sub-statement** of the most recent `person` statement. Lines that begin in column 1 are top-level statements. The exact amount of indentation is not significant; one or more whitespace characters suffices.

Encoding: documents are UTF-8 (no BOM). Line endings are LF or CRLF; a parser MUST accept either. Identifiers and keywords use ASCII; string literals (e.g., display names) may contain any valid UTF-8.

---

## 3. Lexical structure

### 3.1 Comments

A `#` character begins a comment that extends to the end of the line. Comments may appear on their own line or after content on a line. Block comments are not supported in v1.

```
# This is a comment.
person alice name:"Alice"  # comment after content
```

### 3.2 Identifiers

An identifier is `[A-Za-z_][A-Za-z0-9_-]*` — must begin with a letter or underscore; may continue with letters, digits, underscores, and hyphens. Identifiers are case-sensitive; `alice` and `Alice` are different identifiers.

Identifiers are used as person and marriage IDs. They MUST NOT be any of the reserved keywords listed in Section 11.

### 3.3 String literals

A string literal is a sequence of characters enclosed in double quotes: `"Alice Sharma"`. Two escape sequences are recognized:

| Escape | Meaning              |
| ------ | -------------------- |
| `\"`   | Literal double quote |
| `\\`   | Literal backslash    |

No other escape sequences are recognized; a backslash followed by any other character is a lexical error. String contents may contain any valid UTF-8 character including newlines, but newlines inside a string are unconventional and tools may discourage them.

### 3.4 Bare values

Where a value is a single token containing no whitespace, no `:`, no `"`, and no `#`, it MAY be written without quotes. The following are equivalent:

```
gender:female
gender:"female"
```

Use of bare values is conventional for enumerations (`gender:male`, `end_reason:divorce`) and identifier references (`birth m_alice_bob`). Strings that contain a space, colon, hash, or quote MUST be quoted.

### 3.5 Date literals

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

### 3.6 Field syntax

A field is `<name>:<value>` written as a single token group with no whitespace between the name, the colon, and the value. The name is one of the field-name keywords listed in Section 11. The value is a string literal, bare value, date literal, or identifier as appropriate to the field.

```
name:"Alice Sharma"
born:1950-04-12
gender:female
end_reason:divorce
```

Field order within a statement is free.

---

## 4. Top-level statements

### 4.1 Version declaration

The first non-blank, non-comment line of a document MAY be a version declaration:

```
kula 0.1
```

The token following `kula` is a version number in `MAJOR.MINOR` form. A parser that does not recognize the version SHOULD report an error rather than parsing the document. If the version declaration is absent, the version is assumed to be the latest version known to the parser.

A document MUST NOT contain more than one version declaration.

### 4.2 Person statement

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

Sub-statements `birth` and `adoption` are described in Section 5.

Example:

```
person alice name:"Alice Sharma" family:"Sharma" given:"Alice" born:1950-04-12 gender:female
  birth m_ramesh_sita
```

### 4.3 Marriage statement

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

A spousal death does NOT auto-end a marriage. See Section 6.2 (active marriage derivation).

A marriage's two spouse identifiers MUST be distinct (a person cannot marry themselves; see Section 7, rule 4).

Example:

```
marriage m_alice_bob alice bob start:1972-05-12 end:1990-08-01 end_reason:divorce
marriage m_alice_devraj alice devraj start:1992-02-14
```

---

## 5. Person sub-statements

Sub-statements appear as indented lines immediately following a `person` statement. They contribute additional facts to that person's record. Sub-statement order within a person is free. Two sub-statement kinds are defined: `birth` and `adoption`.

### 5.1 birth sub-statement

```
  birth <marriage-id>
```

Declares that this person was biologically born to the union represented by `<marriage-id>`. The two persons named as spouses in that marriage are this person's biological parents.

A person MUST have at most one `birth` sub-statement. Absence is permitted: a person without a `birth` sub-statement has undocumented biological parents and is implicitly a documentation root. No keyword is required to mark a documentation root; the additivity principle ensures that adding a `birth` sub-statement later does not require any change to the existing person line.

The date of biological birth is the person's `born` field; no separate date is needed on the `birth` sub-statement.

Example:

```
person carol name:"Carol Sharma" born:1975-09-03 gender:female
  birth m_alice_bob
```

### 5.2 adoption sub-statement

```
  adoption <marriage-id> start:<date> [end:<date>]
```

Declares that this person was adopted by the couple represented by `<marriage-id>`, becoming effective on `start:<date>`. If the adoption was later terminated, `end:<date>` records when.

A person MAY have zero or more `adoption` sub-statements; multiple adoptions over a lifetime are permitted. Adoptions are permanent unless `end` is given. There is no `end_reason` field on adoption sub-statements in v1.

Example:

```
person ravi name:"Ravi Sharma" born:~1980 gender:male
  adoption m_alice_bob start:1985-06-01
```

---

## 6. Semantics

### 6.1 Reference resolution

A Kula document is parsed in two phases. In the first phase, all top-level `person` and `marriage` statements are collected and indexed by ID. In the second phase, all references — marriage spouses, `birth.<marriage-id>`, and `adoption.<marriage-id>` — are resolved against the indexed declarations. Statements may appear in any order; forward references are permitted.

A reference to an undeclared identifier is an error (Section 7, rule 2).

### 6.2 Active marriage at time T

A marriage M is **active at time T** if and only if:

1. `M.start ≤ T`, and
2. `M.end` is absent OR `M.end > T`, and
3. For each spouse S of M: `S.died` is absent OR `S.died > T`.

Condition 3 means a marriage becomes inactive when either spouse dies, even though the marriage record itself is not modified. This is how Kula reconciles "death does not end the marriage" (the marriage's stored fields are unchanged) with the practical query of who is currently married to whom (which uses this derivation).

### 6.3 Parenthood derivation

The **biological parents** of a person P are:

- If P has a `birth` sub-statement referencing marriage M, then the two spouses of M.
- Otherwise, P's biological parents are undocumented.

The **adoptive parents** of a person P, at time T, are:

- The set of all couples `{spouses of M : M is referenced by an `adoption`sub-statement on P with`start ≤ T` and (`end`absent OR`end > T`)}`.

The set of P's parents at time T is the union of biological parents (if any) and currently-active adoptive parents. P may have more than two parents at a single time T if adopted by an additional couple while biological or earlier adoptive parents remain documented.

### 6.4 Derived kinship terms

Standard kinship terms (sibling, half-sibling, cousin, grandparent, in-law, etc.) are NOT first-class concepts in Kula. They are derivable from the person and marriage graph:

- **Siblings**: persons sharing both biological parents.
- **Half-siblings**: persons sharing exactly one biological parent.
- **Step-siblings**: persons whose parents are spouses in a common marriage but who do not share a biological parent.
- **Grandparents**: parents of parents.
- **Cousins, in-laws, etc.**: derivable similarly.

A v1 conforming implementation is not required to compute these terms; they are noted here for context and as a guide for downstream tools.

---

## 7. Validation rules

A conforming validator MUST report all of the following as errors. A document with any of these errors is **invalid**.

### Structural

1. **Duplicate ID** — no two top-level statements (across `person` and `marriage`) may share an ID.
2. **Unresolved reference** — every `birth` marriage reference, `adoption` marriage reference, and marriage spouse reference must resolve to a declared ID.
3. **Required field missing** — a `person` MUST have `name` and `gender`. A `marriage` MUST have both spouses and `start`. (The positional `id` is also required and is enforced by the grammar; this rule covers the named fields.)
4. **Self-marriage** — a marriage's two spouse references MUST be distinct identifiers.
5. **End consistency** — a marriage's `end` field and `end_reason` field MUST both be present or both absent.

### Temporal impossibilities

For each comparison below, when a date has partial granularity it is treated as a range (e.g., `1925` denotes `1925-01-01..1925-12-31`), and the rule fires only if the comparison is violated for every date pair within the range. The `~` (circa) marker adds a tolerance of ±5 years to the date when the comparison is performed.

6. **Died before born** — `person.died < person.born`.
7. **Marriage end before start** — `marriage.end < marriage.start`.
8. **Adoption end before start** — `adoption.end < adoption.start`.
9. **Marriage before spouse born** — `marriage.start < S.born` for either spouse S.
10. **Spouse already dead at marriage start** — `marriage.start > S.died` for either spouse S.
11. **Bio child born before parent** — `child.born < P.born` for either biological parent P.
12. **Adoption before adopter born** — `adoption.start < P.born` for either adoptive parent P.

### Cycles

13. **Parenthood cycle** — combining all `birth` and `adoption` parent links into a directed graph from child to parent, no person may appear as their own ancestor. (A cycle in this graph is an error.)

### Things explicitly NOT validated in v1

The following are NOT errors, even though they may indicate questionable data. Tools MAY surface them as informational hints, but they are not validation rules:

- A bio child's `born` date falling outside the parents' marriage interval (real-world cases include premarital conception with marriage during pregnancy, and post-divorce births of children conceived during marriage).
- A marriage with a recorded `end` date later than one of the spouses' `died` date.
- A person without optional fields (`born`, `family`, `given`).
- Cultural prohibitions on marriage (incest, same-gotra, sapinda restrictions).
- Reproductive age plausibility ("parent was 9 when child was born").

These exclusions reflect Kula's design priority of recording reality over enforcing legal or cultural norms.

---

## 8. Worked examples

### 8.1 A single couple

```
kula 0.1

person alice name:"Alice Sharma" born:1950-04-12 gender:female
person bob   name:"Bob Sharma"   born:1948-11-30 gender:male

marriage m_alice_bob alice bob start:1972-05-12
```

A married couple, no children, marriage ongoing. Both persons are documentation roots (no `birth` sub-statement).

### 8.2 A nuclear family

```
kula 0.1

person alice name:"Alice Sharma" born:1950-04-12 gender:female
person bob   name:"Bob Sharma"   born:1948-11-30 gender:male
person carol name:"Carol Sharma" born:1975-09-03 gender:female
  birth m_alice_bob

marriage m_alice_bob alice bob start:1972-05-12
```

Carol is the biological child of Alice and Bob's marriage.

### 8.3 A three-generation family with adoption

```
kula 0.1

# ---- Generation 1 (founders) ----
person ramesh name:"Ramesh Sharma" born:1925-03-10 died:2005-08-22 gender:male
person sita   name:"Sita Sharma"   born:1928-07-15 died:2010-11-04 gender:female

marriage m_ramesh_sita ramesh sita start:1948-06-10

# ---- Generation 2 ----
person alice name:"Alice Sharma" born:1950-04-12 gender:female
  birth m_ramesh_sita
person bob   name:"Bob Sharma"   born:1948-11-30 died:2020-03-15 gender:male

marriage m_alice_bob alice bob start:1972-05-12 end:1990-08-01 end_reason:divorce

# ---- Generation 3 ----
person carol name:"Carol Sharma" born:1975-09-03 gender:female
  birth m_alice_bob

person ravi name:"Ravi Sharma" born:~1980 gender:male
  adoption m_alice_bob start:1985-06-01
```

Three generations. Alice is the biological daughter of Ramesh and Sita. Alice and Bob have a biological daughter Carol and an adopted son Ravi. The Alice-Bob marriage ends in divorce.

### 8.4 A polygamous family

```
kula 0.1

person alice  name:"Alice Sharma" born:1950-04-12 gender:female
person devraj name:"Devraj Kumar" born:1948-06-21 gender:male
person meera  name:"Meera"        born:1955-03-08 gender:female

person priya name:"Priya Kumar" born:1994-12-01 gender:female
  birth m_alice_devraj

marriage m_devraj_meera  devraj meera  start:1990-01-01
marriage m_alice_devraj  alice  devraj start:1992-02-14
```

Devraj is concurrently married to Meera and Alice. Priya is the biological daughter of Alice and Devraj.

---

## 9. Edge cases

### 9.1 Founder persons

A person without a `birth` sub-statement is implicitly a documentation root. No keyword is needed:

```
person grandfather name:"Grandfather" gender:male
```

If parents are later learned and added, the existing person line need not change — only a `birth` sub-statement is appended.

### 9.2 Adoption-only persons

A person with an `adoption` sub-statement but no `birth` sub-statement is documented only by their adoptive lineage:

```
person foundling name:"Anika" born:~1985 gender:female
  adoption m_adoptive_couple start:1986-04-01
```

### 9.3 Same-pair remarriage

Two distinct marriages between the same pair of persons receive distinct IDs:

```
marriage m_alice_bob_1 alice bob start:1972-05-12 end:1980-01-01 end_reason:divorce
marriage m_alice_bob_2 alice bob start:1985-06-15
```

A child of either marriage references the appropriate marriage ID via their `birth` sub-statement.

### 9.4 Circa dates

A circa-prefixed date denotes "approximately this date, with imprecision beyond the literal's granularity":

```
person grandfather born:~1925         # somewhere in the mid-1920s
marriage m_g grandfather x start:~1948  # married around 1948
```

Validators apply a ±5-year tolerance to circa dates when comparing.

### 9.5 Marriages ended only by spousal death

A marriage in which one spouse has died but no formal end was recorded simply has no `end` field:

```
person bob   name:"Bob"   died:2020-03-15 gender:male
person alice name:"Alice" gender:female

marriage m_alice_bob alice bob start:1972-05-12
```

The marriage is no longer active at any time after Bob's death (per Section 6.2), but its record is unchanged.

### 9.6 A marriage ended on a known-but-vague date

If you know a marriage ended approximately in 1990 but not the exact date:

```
marriage m_x alice bob start:1972 end:~1990 end_reason:divorce
```

### 9.7 Multiple adoptions

A person may have more than one adoption event, possibly with one ended:

```
person someone name:"Someone" born:1985-01-01 gender:female
  adoption m_first_couple  start:1985-06-01 end:1990-01-01
  adoption m_second_couple start:1992-04-15
```

The first adoption ended in 1990; the second is ongoing.

### 9.8 Bio + adoptive parents coexisting

A person may have biological parents documented AND be adopted by another couple:

```
person ravi name:"Ravi Sharma" born:1980-02-14 gender:male
  birth m_birth_parents
  adoption m_alice_bob start:1985-06-01
```

Both relationships coexist; neither replaces the other.

---

## 10. File conventions

|                |                                            |
| -------------- | ------------------------------------------ |
| File extension | `.kula`                                    |
| Encoding       | UTF-8 (no BOM)                             |
| Line endings   | LF or CRLF (parser MUST accept either)     |
| CLI binary     | `kula` (e.g., `kula validate family.kula`) |

A Kula document MAY be empty (zero statements). Such a document represents the empty family and is valid.

---

## 11. Reserved keywords

The following identifiers are reserved and MUST NOT be used as person or marriage IDs:

```
adoption    birth       born        died        divorce
end         end_reason  family      female      gender
given       kula        male        marriage    name
other       person      start
```

This list (18 keywords) is exhaustive for v1. Future spec versions MAY add reserved keywords; documents that use a now-reserved word as an ID will require updating.

---

## 12. Formal grammar (EBNF)

The following EBNF describes the surface syntax of Kula 0.1. Identifier productions are role-named (`person-id`, `marriage-id`, `person-ref`, `marriage-ref`) — they all share the same lexical form (`identifier`), but the grammar names what each position means. Field productions are enumerated per statement context, so the grammar shows which fields are allowed on which statement kind and what value type each field expects.

Constraints not enforced by the grammar — uniqueness of IDs across the document, reference resolution against declared IDs, field non-duplication within a single statement, presence of fields whose semantic requiredness is conditional (e.g., `end_reason` iff `end`), and the validation rules in Section 7 — are imposed by the language semantics.

```ebnf
document          = [ version-decl NEWLINE ] { line } ;

line              = blank-line | comment-line | top-statement ;
blank-line        = NEWLINE ;
comment-line      = comment NEWLINE ;
comment           = "#" { any-char-except-newline } ;

version-decl      = "kula" version ;
version           = digit { digit } "." digit { digit } ;

(* --- Top-level statements --- *)

top-statement     = person-stmt | marriage-stmt ;

person-stmt       = "person" person-id { person-field } NEWLINE { person-sub-statement } ;
marriage-stmt     = "marriage" marriage-id person-ref person-ref { marriage-field } NEWLINE ;

(* --- Person sub-statements (only valid as indented continuation of a person-stmt) --- *)

person-sub-statement = INDENT ( birth-sub | adoption-sub ) NEWLINE ;
birth-sub            = "birth" marriage-ref ;
adoption-sub         = "adoption" marriage-ref { adoption-field } ;

(* --- Fields, enumerated per context --- *)

person-field      = name-field
                  | family-field
                  | given-field
                  | born-field
                  | died-field
                  | gender-field ;

marriage-field    = start-field
                  | end-field
                  | end-reason-field ;

adoption-field    = start-field
                  | end-field ;

name-field        = "name"       ":" string ;
family-field      = "family"     ":" string ;
given-field       = "given"      ":" string ;
born-field        = "born"       ":" date ;
died-field        = "died"       ":" date ;
gender-field      = "gender"     ":" gender-value ;
start-field       = "start"      ":" date ;
end-field         = "end"        ":" date ;
end-reason-field  = "end_reason" ":" end-reason-value ;

gender-value      = "male" | "female" | "other" ;
end-reason-value  = "divorce" ;

(* --- Identifiers, role-named --- *)

person-id         = identifier ;   (* declares a person; must be unique across all person-id and marriage-id *)
marriage-id       = identifier ;   (* declares a marriage; must be unique across all person-id and marriage-id *)
person-ref        = identifier ;   (* must resolve to a declared person-id *)
marriage-ref      = identifier ;   (* must resolve to a declared marriage-id *)

(* --- Lexical primitives --- *)

string            = '"' { string-char } '"' ;
string-char       = any-char-except-quote-or-backslash
                  | "\\" "\\"
                  | "\\" '"' ;

date              = [ "~" ] year [ "-" month [ "-" day ] ] ;
year              = digit digit digit digit ;
month             = digit digit ;
day               = digit digit ;

identifier        = ( letter | "_" ) { letter | digit | "_" | "-" } ;
                    (* Identifier MUST NOT match any reserved keyword listed in Section 11. *)
letter            = "A" | "B" | ... | "Z" | "a" | "b" | ... | "z" ;
digit             = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;

NEWLINE           = "\n" | "\r\n" ;
INDENT            = horizontal-whitespace { horizontal-whitespace } ;
horizontal-whitespace = " " | "\t" ;
```

Whitespace separates tokens within a line and is otherwise ignored, except that:

- Inside any field production (e.g., `name-field`, `start-field`), no whitespace is allowed between the field-name keyword, `:`, and the value.
- Inside a `date`, no whitespace is allowed between any of `~`, `year`, `-`, `month`, `-`, `day`.
- Leading whitespace on a line that is not a top-level statement constitutes `INDENT` and binds the line as a sub-statement of the most recent `person-stmt`.

---

## 13. Versioning policy

The Kula language is versioned by a `MAJOR.MINOR` scheme.

- **MINOR version increments** add backward-compatible features: new optional fields, new enumeration values, new statement kinds, additional sub-statement kinds. A document valid at version `0.MINOR` remains valid at `0.MINOR+1`.
- **MAJOR version increments** may make breaking changes: removing fields, renaming keywords, changing semantics. A `0.x` document is not guaranteed to be valid in `1.x`.

A document with `kula 0.1` MUST be parsed and validated according to this specification. A parser encountering a higher version it does not know SHOULD report an error rather than silently parse the document under different rules.

---

_End of Kula 0.1 specification._
