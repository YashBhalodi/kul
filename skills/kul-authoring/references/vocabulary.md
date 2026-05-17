# Kinship vocabulary

This reference is the load-bearing one for translation. Everyday English (and other natural-language) kinship terms map onto Kul's primitives in one of three ways:

- **Declared** — you write a statement (`person`, `marriage`) or sub-statement (`birth`, `adoption`) directly.
- **Derived** — there is no statement for it; the relation falls out of the person + marriage + birth/adoption graph.
- **Not modeled** — the language deliberately doesn't capture this concept in v1; record what *is* modeled and either omit the rest or add a comment.

Sourced from the kinship section of [`CONTEXT.md`](../../../CONTEXT.md) and the semantics chapter of the spec ([`spec/06-semantics.md`](../../../spec/06-semantics.md)). If you ever find this file disagreeing with `CONTEXT.md`, `CONTEXT.md` wins — open an issue.

## The four declared primitives

| Concept    | Statement                                                                 | One-line meaning                                                                       |
| ---------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Person     | `person <id> name:"…" gender:… [born:…] [died:…] [family:…] [given:…]`    | A declared individual. `id` is a stable lowercase handle; `name` and `gender` required. |
| Marriage   | `marriage <id> <spouse_a> <spouse_b> start:… [end:…] [end_reason:divorce]`| A declared union between two declared persons. Spouse order has no meaning.            |
| Birth      | `birth <marriage-id>` (indented under a `person`)                         | This person is the biological child of the spouses of the named marriage.              |
| Adoption   | `adoption <marriage-id> start:… [end:…]` (indented under a `person`)      | This person was adopted into the named marriage on `start:`; permanent unless `end:`.  |

That is the entire surface. Every other kinship concept either reduces to a combination of these four, or it isn't in Kul v1.

## English → Kul mapping table

| Natural-language term            | Declared / derived / not modeled | How it shows up                                                                                   |
| -------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------- |
| **Spouse / husband / wife / partner-in-marriage** | **Declared**                     | A `marriage` with the two persons as `<spouse_a>` and `<spouse_b>`.                               |
| **Ex-husband / ex-wife / divorced spouse**        | **Declared**                     | `marriage … start:… end:<date> end_reason:divorce`.                                               |
| **Widow / widower**              | **Derived**                      | A surviving spouse of a `marriage` whose other spouse has a `died:` date and the marriage has no `end:`. Per spec §6.2, the marriage becomes inactive but is unchanged. |
| **Engaged / fiancé / fiancée**   | **Not modeled**                  | Kul models marriages only. If the engagement is load-bearing, record it as a comment until the marriage is solemnized. |
| **Cohabiting partner / common-law partner**       | **Not modeled**                  | No declaration. Comment if needed.                                                                |
| **Child / son / daughter**       | **Derived**                      | A `person` with a `birth` (biological) or `adoption` (adoptive) sub-statement pointing at the parents' marriage. There is no `child` keyword and no `parent` field. |
| **Biological child**             | **Derived (declared via `birth`)** | The child has a `birth <marriage-id>` sub-statement.                                              |
| **Adopted child**                | **Derived (declared via `adoption`)** | The child has an `adoption <marriage-id> start:…` sub-statement.                                  |
| **Parent / father / mother**     | **Derived**                      | Spouse of a marriage referenced by some person's `birth` (biological) or `adoption` (adoptive) sub-statement. |
| **Step-father / step-mother**    | **Derived**                      | A spouse of a parent's *other* marriage (not the marriage the child links to via `birth`/`adoption`). |
| **Adoptive father / adoptive mother** | **Derived**                  | A spouse of the marriage referenced by the child's `adoption` sub-statement.                      |
| **Sibling / brother / sister**   | **Derived**                      | Two persons whose `birth` sub-statements reference the same marriage id.                          |
| **Half-sibling**                 | **Derived**                      | Two persons who share exactly one biological parent — i.e. one spouse of person A's `birth` marriage is also a spouse of person B's `birth` marriage, but the marriages are different. |
| **Step-sibling**                 | **Derived**                      | Two persons who do not share a biological parent, but whose parents are spouses in a common marriage. |
| **Twin (identical or fraternal)** | **Not modeled (partially derived)** | Modeled as two siblings with the same `born:` date. Twin-ness as such is not a first-class concept. |
| **Grandparent / grandchild**     | **Derived**                      | Parent-of-parent and child-of-child. Walk the `birth`/`adoption` links two steps.                 |
| **Great-grandparent / great-grandchild** | **Derived**              | Walk three steps.                                                                                 |
| **Uncle / aunt**                 | **Derived**                      | A sibling of a parent (and, by extension, the spouse of such a sibling — an "uncle/aunt by marriage" is a spouse in an uncle/aunt's marriage). |
| **Cousin (first, second, …)**    | **Derived**                      | Children of siblings (first); grandchildren of siblings (second); etc.                            |
| **Niece / nephew**               | **Derived**                      | A child of a sibling.                                                                             |
| **In-law (mother-in-law, father-in-law, brother-in-law, sister-in-law, etc.)** | **Derived** | Relations to / between members of a spouse's family. All compose out of `marriage` + `birth`.   |
| **Founder / patriarch / matriarch / root ancestor** | **Derived** (implicit)    | A `person` with no `birth` sub-statement. No keyword marks them as a founder; the absence is the marker. Adding a `birth` later is a one-line append (the additivity principle). |
| **Foundling / abandoned child / unknown bio parents** | **Derived** (implicit)  | A `person` with no `birth` sub-statement but with one or more `adoption` sub-statements.          |
| **Polygamous spouse / co-wife / co-husband** | **Declared**            | Multiple `marriage` statements for the same person; spec permits concurrent marriages. See `examples/04-polygamous-family/`. |
| **Same-pair remarriage**         | **Declared**                     | Two distinct `marriage` statements with the same `<spouse_a>` and `<spouse_b>` and different ids — e.g. `m_alice_bob_1` and `m_alice_bob_2`. See [`spec/09-edge-cases.md`](../../../spec/09-edge-cases.md) §9.3. |
| **Friend / godparent / mentor / patron** | **Not modeled**          | Kul v1 covers kinship only. Comment if load-bearing.                                              |
| **Donor / surrogate / gamete-donor parent** | **Not modeled**       | Per [`docs/vision.md`](../../../docs/vision.md). Use comments to record context.                  |
| **Single parent (legally)**      | **Not modeled directly**         | Every child links to a marriage (two parents). If only one parent is documented in real life, record the child's bio link to the marriage that *did* exist and treat the missing partner as undocumented. |

## Things that look declared but are derived

These are the most common authoring mistakes — places where prose suggests a direct declaration but the language treats them as derivations.

- **Don't write a `child` field.** Children are reached by their own `birth` / `adoption` sub-statement.
- **Don't write a `parent` field.** Parents are reached by the marriage spouses on the child's `birth` / `adoption` link.
- **Don't write a `sibling` declaration.** Siblings share a `birth` link.
- **Don't write `step:` or `half:` modifiers.** Step / half are derivations from the multi-marriage graph.
- **Don't try to write `gender:nonbinary` or other unmodeled values.** The v1 enum is `male | female | other` — `other` covers everything not in `male`/`female`. (See [`spec/04-top-level-statements.md`](../../../spec/04-top-level-statements.md) §4.1.)
- **Don't try to write `end_reason:death`.** Spousal death does not end the marriage; the v1 vocabulary for `end_reason` is `divorce` only. If a marriage was only ever interrupted by death, simply omit `end:` and `end_reason:`. (See [`spec/05-person-sub-statements.md`](../../../spec/05-person-sub-statements.md), [`spec/06-semantics.md`](../../../spec/06-semantics.md) §6.2.)

## Gender field

`gender` is **required** on every `person`. The v1 enum is exactly three values:

| Value    | Used for                                                |
| -------- | ------------------------------------------------------- |
| `male`   | Persons referred to with masculine kinship terms.       |
| `female` | Persons referred to with feminine kinship terms.        |
| `other`  | Persons not represented by `male` or `female`, including unknown / unstated. |

When prose doesn't state a gender:

- If a name strongly implies one in a context you're confident about, you may use that gender — but flag the inference with a `#` comment on the line so a human reviewer can confirm.
- If you're not confident, use `gender:other` and a `#` comment noting the prose was silent.
- **Never** make up a name to imply a gender; prefer recording the prose's wording in the `name:` field exactly.

## Date granularity

The `born:`, `died:`, `start:`, `end:` fields all take a [date literal](../../../spec/03-lexical-structure.md#35-date-literals): `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`, optionally prefixed with `~` for circa (±5-year tolerance during validation).

Match the granularity of the prose:

| Prose                                  | Literal              |
| -------------------------------------- | -------------------- |
| "born March 15, 1985"                  | `born:1985-03-15`    |
| "born in March 1985"                   | `born:1985-03`       |
| "born in 1985"                         | `born:1985`          |
| "born around 1985" / "born circa 1985" | `born:~1985`         |
| "born in the mid-1980s"                | `born:~1985`         |
| "born sometime in the 1980s"           | `born:~1985` (and add a `#` comment noting the decade) |
| "birth year unknown"                   | omit `born` entirely |

There is no syntax for unknown dates; absence is the canonical way to express "not recorded." (Spec §3.5.)

## Identifier conventions

Identifiers (the ids on `person <id>` and `marriage <id>`) are `[A-Za-z_][A-Za-z0-9_-]*` and must not be one of the 17 reserved keywords ([`spec/11-reserved-keywords.md`](../../../spec/11-reserved-keywords.md)).

Conventions the example corpus follows — match them when authoring fresh `.kul`:

- **Person ids** — lowercase given name (`alice`, `bob`, `ramesh`, `priya`). Disambiguate same-name persons with a generational suffix (`alice_sr`, `alice_jr`) or a family-side suffix (`ravi_paternal`, `ravi_maternal`).
- **Marriage ids** — `m_<spouse_a>_<spouse_b>` (`m_alice_bob`, `m_ramesh_sita`). For same-pair remarriage, suffix with `_1`, `_2`: `m_alice_bob_1`, `m_alice_bob_2` ([`spec/09-edge-cases.md`](../../../spec/09-edge-cases.md) §9.3).
- **Ids stay short and ASCII**. Display names go in `name:` and can be any UTF-8.
