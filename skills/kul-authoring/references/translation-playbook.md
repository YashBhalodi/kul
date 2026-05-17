# Translation playbook

How to turn a natural-language family narrative into idiomatic `.kul`. Load this whenever you start an NL→Kul task. It covers:

1. [Ambiguity-handling rules](#ambiguity-handling-rules) — what to do when the prose is incomplete, vague, or inconsistent.
2. [Five paired NL↔.kul examples](#five-paired-examples) — the most common translation shapes you'll encounter.

If a rule in this file appears to disagree with the spec or with `CONTEXT.md`, those win. Open an issue.

## Ambiguity-handling rules

The single discipline behind every rule is the **additivity principle** ([`spec/13-versioning-policy.md`](../../../spec/13-versioning-policy.md), [`CONTEXT.md`](../../../CONTEXT.md)): never make a guess that would have to be *unwritten* if a more accurate fact arrived. Omit instead of guess; flag inferences instead of burying them.

### R1. Missing dates → omit

If the prose doesn't say when something happened, **don't make a date up**. Omit the field.

- "Born in March 1985" → `born:1985-03`
- "Born in the 1980s" → `born:~1985` (and add a trailing `#` comment noting the decade)
- "Year unknown" / not mentioned → omit `born:` entirely
- "Married a long time ago" → omit `start:` is **not an option** (`start:` is required on `marriage`). Use `start:~<rough-year>` and add a `# circa: prose was vague` comment.

There is no `unknown` date literal. Absence of a date is the canonical "not recorded" signal.

### R2. Missing gender → don't infer from name; use `gender:other`

`gender:` is **required** on every `person`, so you cannot omit it. The v1 enum is `male | female | other`.

- Prose names the gender ("his daughter", "her husband") → use the corresponding `male`/`female`.
- Prose is silent and the name is culturally unambiguous to you (e.g. "Margaret" in a Western context) → **still flag the inference** with a `# gender inferred from name; confirm` comment. Names are unreliable across cultures.
- Prose is silent and the name is ambiguous → `gender:other` with a `# gender unstated in source` comment.

The principle: name-based gender inference is a guess that the additivity principle penalizes. A human reviewing the file should know which gender values come from the prose and which from your inference.

### R3. Unnamed individuals → placeholder id, prose name, mark them

Sometimes prose introduces a person without naming them: "Ramesh's first wife", "the youngest of three sisters", "a cousin in Bombay". You still need them as a `person` to declare any marriages or births that depend on them.

- Pick a descriptive id: `first_wife_of_ramesh`, `youngest_sister_of_alice`, `bombay_cousin`.
- Put the closest prose phrasing in `name:` ("Ramesh's first wife", quoted).
- Use `gender:other` (or the gender the prose makes explicit) and add a `# unnamed in source` comment.

Don't invent a personal name. The id is a stable handle; the `name:` field can be edited later when more is known.

### R4. Derived relations in prose → resolve to declared primitives

When prose names a derived relation ("Alice's uncle Ravi", "Bob's grandfather"), your job is to **walk it back** to the persons, marriages, and birth/adoption links that make it derivable. Then declare those primitives.

- "Alice's uncle Ravi, on her mother's side" → Ravi is a sibling of Alice's mother. To declare this:
  1. Make sure Alice's mother is declared (which she must already be, for Alice's `birth` to point at her parents' marriage).
  2. Declare Ravi as a `person`.
  3. Declare Alice's *maternal grandparents'* marriage (or reuse it if it already exists).
  4. Add a `birth <grandparents-marriage-id>` sub-statement to both Alice's mother and to Ravi.

Now "uncle" falls out of the graph: Ravi shares both biological parents with Alice's mother (so he's her sibling), and Alice's mother is Alice's parent.

This often unearths the next thing the prose didn't say — like the maternal grandparents' marriage. Capture what you can; comment what you can't.

### R5. Implicit marriages → declare them explicitly

If prose says "John and Mary had a son Tom" without ever mentioning that John and Mary were married, **you must still declare a `marriage`**. The language has no other way to bind two persons as biological parents — every `birth` sub-statement points at a marriage id.

- Pick an id: `m_john_mary`.
- Use the best date you have for `start:`. If the prose only tells you they had a son in 1970, `start:~1970` is defensible (a marriage that produced a 1970 child probably started around or before 1970; the ±5-year tolerance accommodates this).
- Add a `# marriage implied by parenthood; not stated in source` comment.

The alternative — declaring Tom with no `birth` link — loses the parenthood information entirely. The whole point of `birth` is to record bio parents; the marriage exists to be the target of `birth`.

If the prose explicitly says the parents were not married (a non-marital union), you have two choices: (a) declare a marriage anyway and comment the non-marital status, or (b) declare the parents but no marriage and accept that Tom's bio parents are not derivable. Both are imperfect — Kul v1 doesn't model non-marital partnerships. Default to (a) because it preserves derivability; flag with a comment.

### R6. Conflicting accounts → represent both as comments, pick one as canonical

When two sources disagree ("Aunt Priya said grandfather was born in 1925; the family bible says 1923"), the language doesn't give you a way to record both as data. Pick one for the field and put the other in a comment:

```
person grandfather  name:"Grandfather"  gender:male  born:1925  # family bible says 1923
```

If neither account is more authoritative, default to the wider-bounds one (or use `~` to widen): `born:~1924` covers both within the ±5-year tolerance.

### R7. Order of operations when reading a paragraph

A workable scanning order for a chunk of prose:

1. Highlight every **proper noun** that names a person. Each becomes a `person`.
2. Highlight every **kinship verb / noun** that implies a marriage (`married`, `husband of`, `wife of`, `spouse`, `divorced`, `widowed`). Each becomes a `marriage`.
3. Highlight every **parenthood verb / noun** (`son of`, `daughter of`, `child of`, `born to`, `mother of`, `father of`, `parents of`). Each becomes a `birth` sub-statement.
4. Highlight every **adoption verb / noun** (`adopted`, `adoptive`, `raised by`). Each becomes an `adoption` sub-statement.
5. Highlight every **derived term** (`uncle`, `cousin`, `step-mother`, `half-sibling`) and apply rule R4 above to resolve them.

Persons come first because marriages and births reference them. Marriages come before births because births reference marriages.

### R8. Comments are first-class context

The formatter preserves comments verbatim (`spec/15-formatter-rules.md` §15.7). Use them liberally to record:

- Inferences you made (R2, R5).
- Where the prose was vague (R1).
- Conflicting accounts (R6).
- The provenance of an unusual decision.

Comments don't affect validation or export. A `.kul` file dense with comments is still idiomatic; an undocumented inference is not.

## Five paired examples

Each example pairs ~30–60 lines of prose with its translated `.kul`. The leading bullet in each section names the playbook rule(s) the example exemplifies.

### Example 1 — Simple nuclear family

**Exemplifies:** R7 (order of operations), R4 (derived "daughter" resolved to `birth`).

This mirrors [`examples/02-nuclear-family/nuclear-family.kul`](../../../examples/02-nuclear-family/nuclear-family.kul).

> **Prose**
>
> Alice Sharma was born on April 12, 1950. Her husband Bob Sharma was born on November 30, 1948. They married on May 12, 1972. They had one daughter, Carol Sharma, born September 3, 1975.

Scan: three persons (Alice, Bob, Carol), one marriage (Alice + Bob, 1972), one parenthood (Carol born to the Alice + Bob marriage). "Daughter" is derived — it becomes Carol's `birth` sub-statement pointing at Alice and Bob's marriage.

```
person alice  name:"Alice Sharma"  gender:female  born:1950-04-12
person bob    name:"Bob Sharma"    gender:male    born:1948-11-30
person carol  name:"Carol Sharma"  gender:female  born:1975-09-03
  birth m_alice_bob

marriage m_alice_bob alice bob  start:1972-05-12
```

The marriage id `m_alice_bob` follows the conventional `m_<spouse_a>_<spouse_b>` shape. Statement order is free — the marriage is declared after Carol's `birth m_alice_bob` references it; forward references resolve cleanly.

### Example 2 — Three-generation family with derived relations

**Exemplifies:** R4 (derived "grandparents", "uncle" resolved), R5 (divorce captured with `end_reason`), R1 (circa date).

This mirrors [`examples/03-three-generations/three-generations.kul`](../../../examples/03-three-generations/three-generations.kul).

> **Prose**
>
> Ramesh and Sita Sharma were Alice's parents. Ramesh was born March 10, 1925; Sita was born July 15, 1928. They married in June 1948 — the 10th. Ramesh died in August 2005; Sita died in November 2010.
>
> Their daughter Alice (born April 12, 1950) married Bob Sharma (born November 30, 1948) on May 12, 1972. The marriage ended in divorce on August 1, 1990. Bob died on March 15, 2020.
>
> Alice and Bob had a daughter Carol, born September 3, 1975. They also adopted a son, Ravi, in June 1985 — Ravi himself had been born around 1980, though no one in the family remembers his exact birthday.

Scan:

- Persons: Ramesh, Sita, Alice, Bob, Carol, Ravi (six).
- Marriages: Ramesh + Sita (1948), Alice + Bob (1972, ended 1990 divorce).
- Births: Alice born to Ramesh + Sita; Carol born to Alice + Bob.
- Adoption: Ravi adopted by Alice + Bob, 1985.
- Derived relations the prose uses: "their daughter" (twice — Alice is Ramesh + Sita's daughter; Carol is Alice + Bob's daughter), "adopted son". All become sub-statements.
- "Around 1980" for Ravi's birth → `born:~1980` (R1).
- The Alice + Bob divorce captures both `end:` and `end_reason:divorce` (rule KUL-R05 requires them together).

```
# ---- Generation 1 (founders) ----
person ramesh  name:"Ramesh Sharma"  gender:male    born:1925-03-10  died:2005-08-22
person sita    name:"Sita Sharma"    gender:female  born:1928-07-15  died:2010-11-04

marriage m_ramesh_sita ramesh sita  start:1948-06-10

# ---- Generation 2 ----
person alice  name:"Alice Sharma"  gender:female  born:1950-04-12
  birth m_ramesh_sita
person bob    name:"Bob Sharma"    gender:male    born:1948-11-30  died:2020-03-15

marriage m_alice_bob alice bob  start:1972-05-12  end:1990-08-01  end_reason:divorce

# ---- Generation 3 ----
person carol  name:"Carol Sharma"  gender:female  born:1975-09-03
  birth m_alice_bob
person ravi   name:"Ravi Sharma"   gender:male    born:~1980
  adoption m_alice_bob  start:1985-06-01
```

The generation comments are conventional in larger files — they map directly to how the prose narrated the family.

### Example 3 — Implicit marriage / unstated union

**Exemplifies:** R5 (implicit marriage declared with a comment), R1 (date-vagueness resolved with `~`), R2 (gender inferred and flagged).

> **Prose**
>
> John and Mary had three children: Tom (born 1962), Susan (born 1965), and Peter (born 1968). The narrative doesn't say when John and Mary were married, or whether they ever were — but they raised the children together. John died in 1990; Mary died in 2005.

Scan:

- Persons: John, Mary, Tom, Susan, Peter.
- The prose **never declares a marriage** between John and Mary, only that the three children belonged to them. Per R5, we must declare a marriage anyway — `birth` only points at marriages, so without one we'd lose the parenthood information.
- The earliest evidence we have for the marriage is Tom's 1962 birth, so `start:~1962` is defensible. Add a comment noting the inference.
- "John" and "Mary" are culturally common names; we tentatively infer gender from them but flag the inference (R2).

```
person john   name:"John"   gender:male    died:1990  # gender inferred from name
person mary   name:"Mary"   gender:female  died:2005  # gender inferred from name

# Marriage not explicitly stated in source; implied by shared parenthood of Tom/Susan/Peter.
marriage m_john_mary john mary  start:~1962

person tom    name:"Tom"    gender:male    born:1962
  birth m_john_mary
person susan  name:"Susan"  gender:female  born:1965
  birth m_john_mary
person peter  name:"Peter"  gender:male    born:1968
  birth m_john_mary
```

The two inference comments (gender from name; marriage from shared parenthood) are how a human reviewer learns which facts came from the prose and which from your guesses. Without those comments, the file looks like an authoritative record.

### Example 4 — Missing or circa dates

**Exemplifies:** R1 (date granularity matched to prose; circa for approximations; field omitted for unknowns), R6 (conflicting accounts captured in a comment).

> **Prose**
>
> Our grandfather was born sometime in the mid-1920s — Aunt Priya remembers him saying "1925" but the family bible says 1923. He married our grandmother around 1948; their exact wedding date isn't recorded anywhere. She died in March 1995; he outlived her, dying on October 12, 2010. They had two children: a son born September 1950 (we have only the month) and a daughter whose birth year nobody remembers.

Scan:

- Grandfather: `born` is approximate. Aunt Priya says 1925, the bible says 1923. Per R6, pick one for the field and comment the other. `born:~1924` covers both within ±5y, but `~1925` with a `# bible says 1923` comment is more faithful to the dominant source.
- Grandmother: died March 1995 — `died:1995-03`. No `born` (not in prose) — R1.
- Marriage: around 1948 → `start:~1948`. No `end:` because neither divorce nor a stated end appears in prose; spousal death does not end the marriage.
- Son: born September 1950 → `born:1950-09` (year-month granularity).
- Daughter: no birth year → omit `born:` (R1). Use `# birth year unknown` so a reader doesn't think we forgot.

```
person grandfather  name:"Grandfather"  gender:male    born:~1925  died:2010-10-12  # bible says 1923
person grandmother  name:"Grandmother"  gender:female                died:1995-03

marriage m_grandparents grandfather grandmother  start:~1948  # exact wedding date not recorded

person son       name:"Son"       gender:male    born:1950-09        # day not recorded
  birth m_grandparents
person daughter  name:"Daughter"  gender:female                       # birth year unknown
  birth m_grandparents
```

Note that "son" and "daughter" are used here as person ids only because the prose never names them. For a real document we'd choose more specific ids if any name surfaced.

### Example 5 — Cross-file split for a large family

**Exemplifies:** Multi-file partitioning (see [`multi-file.md`](./multi-file.md)), R1 (circa date used for a great-grandchild).

This mirrors the shape of [`examples/07-multi-file-extended-family/`](../../../examples/07-multi-file-extended-family/).

> **Prose**
>
> Ramesh and Sita Patel were the founders of our line. Ramesh was born April 10, 1928, and died December 3, 2010. Sita was born September 22, 1931, and died June 14, 2018. They married February 18, 1952.
>
> They had two children: Alice (born July 19, 1955) and Dev (born March 28, 1958). Alice married Bob Khan (born November 2, 1953) on September 15, 1978. Dev married Eva Singh (born December 4, 1959) on April 22, 1982.
>
> Alice and Bob had two daughters, Carol (born June 8, 1981) and Diya (born October 30, 1984). Dev and Eva had two children too: Farid (born January 17, 1985) and Gita (whose birth year is approximately 1988 — nobody is sure of the month).

The prose covers three generations with seven persons in Gen 2/3, plus the two founders — nine persons total. That's at the lower end of where splitting earns its keep, but the generational structure is clean and partitioning by generation gives a readable shape.

`kul.yml`:

```yaml
kul: "0.1"
```

`01-founders.kul`:

```
# Generation 1 — the founders.

person ramesh  name:"Ramesh Patel"  gender:male    born:1928-04-10  died:2010-12-03
person sita    name:"Sita Patel"    gender:female  born:1931-09-22  died:2018-06-14

marriage m_ramesh_sita ramesh sita  start:1952-02-18
```

`02-parents.kul`:

```
# Generation 2 — Alice and Dev plus their spouses and marriages.

person alice  name:"Alice Patel"  gender:female  born:1955-07-19
  birth m_ramesh_sita
person bob    name:"Bob Khan"     gender:male    born:1953-11-02

marriage m_alice_bob alice bob  start:1978-09-15

person dev  name:"Dev Patel"  gender:male    born:1958-03-28
  birth m_ramesh_sita
person eva  name:"Eva Singh"  gender:female  born:1959-12-04

marriage m_dev_eva dev eva  start:1982-04-22
```

`03-grandchildren.kul`:

```
# Generation 3 — children of the Gen 2 marriages.

person carol  name:"Carol Patel"  gender:female  born:1981-06-08
  birth m_alice_bob
person diya   name:"Diya Patel"   gender:female  born:1984-10-30
  birth m_alice_bob

person farid  name:"Farid Patel"  gender:male    born:1985-01-17
  birth m_dev_eva
person gita   name:"Gita Patel"   gender:female  born:~1988      # year approximate
  birth m_dev_eva
```

Key observations:

- The marriage ids `m_ramesh_sita`, `m_alice_bob`, `m_dev_eva` are declared in earlier files; the `birth` sub-statements in later files reference them by bare id. No `import` is needed — the whole directory is one logical namespace ([`multi-file.md`](./multi-file.md)).
- Gita's circa-1988 birth uses `~` (R1).
- Each file has a one-line header comment naming the slice it covers.
- The Patel-Khan-Singh surname mix shows up purely in `name:` strings; partitioning is by generation, not by surname (which would scatter the Khan-by-marriage spouse).

If the prose grew to include Gita's eventual marriage and her own children, you'd add a `04-great-grandchildren.kul` (or a similar slice). You wouldn't rewrite any existing file — additivity.
