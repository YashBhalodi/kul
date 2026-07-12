# Kinship-term inventory — Gujarati + English over the `RelationshipDescriptor`

Research asset for issue [#278](https://github.com/YashBhalodi/kul/issues/278), part of the
[query-UX wayfinder map (#275)](https://github.com/YashBhalodi/kul/issues/275). It is the
reality-check for the phrasing-layer architecture decision
([#279](https://github.com/YashBhalodi/kul/issues/279)): every lexicalized Gujarati and English
kinship term, expressed against the descriptor's dimensions ([ADR-0026](./adr/0026-relationship-descriptor-and-path-identity.md)),
so the mapping-entry shape can be judged against what it must express.

**Lifecycle**: transient, like a PRD — when the phrasing layer ships, this inventory graduates
into its `en`/`gu` term data (and the tests that pin it), and this file is deleted. Findings
load-bearing beyond the epic get lifted into the phrasing-layer ADR first.

## How to read the mapping

The descriptor's normalized dimensions (ADR-0026, [`descriptor.rs`](../crates/kul-core/src/query/descriptor.rs)):

| Dimension | Values |
| --- | --- |
| `classification` | `self` \| lineal `{role, generations}` \| collateral `{up, down, cousinDegree, removed}` |
| `edgeNature` | `blood` \| `adoptive` (any adoption edge on the path) |
| `affinity` | `blood` \| `step` \| `inLaw` (from `across` hops and their position) |
| `sharing` | `full` \| `half` \| `notApplicable` (sibling-junction parent sets) |
| `side` | `maternal` \| `paternal` \| `other` \| `both` \| `notApplicable` (initial ascent) |
| `seniority` / `apexSeniority` | `elder` \| `younger` \| `unknown` \| `notApplicable` |
| `egoGender` / `alterGender` | `male` \| `female` \| `other` |

Plus the lossless **path backbone** — ordered hops, each carrying the person landed on and that
person's gender (and marriage id/status for `across` hops).

Each term row gives the **path shape** (hop sequence from ego) and the **selecting keys** — the
facts that pick this term over its neighbours. Keys marked **⚠ backbone** are *not* normalized
descriptor fields: they are the gender (or position) of a specific hop, recoverable only from the
path backbone. These ⚠ rows are the architectural payload of this document — see
[Findings](#findings-for-the-mapping-entry-shape-279).

Transliteration is informal (long vowels marked); script per common usage.

## 1. Lineal ancestors

| Term | Script | Gloss | Path shape | Selecting keys |
| --- | --- | --- | --- | --- |
| bā / mā / mummy | બા / મા | mother | `up` | lineal ancestor gen 1, alter female |
| bāpuji / pappā / pitā | બાપુજી / પપ્પા / પિતા | father | `up` | lineal ancestor gen 1, alter male |
| dādā | દાદા | father's father | `up·up` | gen 2, side `paternal`, alter male |
| dādī | દાદી | father's mother | `up·up` | gen 2, side `paternal`, alter female |
| nānā / nānā-bāpā | નાના | mother's father | `up·up` | gen 2, side `maternal`, alter male |
| nānī / nānī-mā | નાની | mother's mother | `up·up` | gen 2, side `maternal`, alter female |
| par-dādā / par-dādī | પરદાદા / પરદાદી | father-side great-grandparent | `up·up·up` | gen 3, side `paternal`, alter gender |
| par-nānā / par-nānī | પરનાના / પરનાની | mother-side great-grandparent | `up·up·up` | gen 3, side `maternal`, alter gender |

- `side` derives from the *first* ascent hop, so dādā/nānā key cleanly on it. A direct parent has
  `side = notApplicable` (the code pins this: "your mother is not your maternal side") — and
  indeed Gujarati parent terms carry no side. ✓
- The `par-` prefix is productive for exactly one extra generation; beyond gen 3 there is no
  lexicalized term (`vaḍvā` / "pūrvajo" = ancestors generically) → compositional fallback.
- Gujarati does **not** lexicalize which *intermediate* line a gen-3 ancestor runs through
  (father's mother's father has no distinct term) — side of first hop is all the lexicon uses. ✓

## 2. Lineal descendants

| Term | Script | Gloss | Path shape | Selecting keys |
| --- | --- | --- | --- | --- |
| dīkro / putra | દીકરો / પુત્ર | son | `down` | lineal descendant gen 1, alter male |
| dīkrī / putrī | દીકરી / પુત્રી | daughter | `down` | lineal descendant gen 1, alter female |
| pautra / pautrī | પૌત્ર / પૌત્રી | **son's** son / daughter | `down·down` | gen 2, alter gender, **⚠ backbone: first `down` hop lands on a male** |
| dohitra / dohitrī | દોહિત્ર / દોહિત્રી | **daughter's** son / daughter | `down·down` | gen 2, alter gender, **⚠ backbone: first `down` hop lands on a female** |
| prapautra / prapautrī | પ્રપૌત્ર | great-grandson (son's line) | `down·down·down` | gen 3, alter gender, ⚠ linking genders |

- **Finding**: `side` is `notApplicable` on every descendant path (no initial ascent), yet
  Gujarati splits grandchildren by the linking *child's* gender — pautra vs dohitra. This is the
  first place normalized fields cannot select the term; the backbone's first-hop gender can.
  (Colloquial speech also just says *dīkrā-no dīkro* — "son's son" — which is itself a genitive
  walk down the backbone.)
- Beyond gen 3: no lexicalized terms → compositional fallback.

## 3. Siblings

| Term | Script | Gloss | Path shape | Selecting keys |
| --- | --- | --- | --- | --- |
| bhāī | ભાઈ | brother | `up·down` | collateral {1,1,0,0}, affinity `blood`, alter male |
| bahen / ben | બહેન | sister | `up·down` | collateral {1,1,0,0}, affinity `blood`, alter female |
| moṭā bhāī / moṭī ben | મોટા ભાઈ | elder brother / sister | `up·down` | + `seniority = elder` |
| nānā bhāī / nānī ben | નાના ભાઈ | younger brother / sister | `up·down` | + `seniority = younger` |
| savkā bhāī / savkī bahen | સાવકા ભાઈ | step-sibling (colloquially also half-) | `up·across·down` | affinity `step` (and colloquially `sharing = half`) |

- Sibling seniority in Gujarati is a **compositional modifier** (moṭā/nānā adjectives), not a
  distinct lexeme (unlike, say, Tamil aṇṇan/tampi). When `seniority = unknown`, the honest
  rendering is the unmarked term (bhāī, never a guessed moṭā bhāī) — the descriptor's explicit
  `unknown` supports exactly this. ✓
- Full siblings have `side = both` (couple apex) — and Gujarati indeed has no "maternal brother"
  term. ✓ For half-siblings `side` reports the linking parent's gender; Gujarati doesn't
  lexicalize that either — *savkā* covers it.
- **Finding**: colloquial Gujarati *savkā* conflates the descriptor's `affinity = step` and
  `sharing = half` (both "not my full sibling"). The descriptor's split (ADR-0026) is strictly
  richer; the `gu` mapping may collapse the two regions onto one term — a many-to-one mapping,
  which is fine. The reverse (one region needing two terms) never occurs.

## 4. Parents' siblings and their spouses — the five-way split

The classic tier where English says "uncle/aunt" and Gujarati has ten terms. Path shape for the
blood relative: `up·up·down` (collateral {2,1,0,1}); the spouse adds a final `across`.

| Term | Script | Gloss | Selecting keys |
| --- | --- | --- | --- |
| kākā | કાકા | father's brother (esp. younger) | side `paternal`, affinity `blood`, alter male (+ `apexSeniority = younger` in careful usage) |
| moṭā bāpā / moṭā kākā | મોટા બાપા | father's **elder** brother | side `paternal`, blood, alter male, `apexSeniority = elder` |
| kākī | કાકી | kākā's wife | side `paternal`, affinity `inLaw`, alter female |
| foī / fai | ફોઈ / ફઈ | father's sister | side `paternal`, blood, alter female |
| fuvā | ફુવા | foī's husband | side `paternal`, `inLaw`, alter male |
| māmā | મામા | mother's brother | side `maternal`, blood, alter male |
| māmī | મામી | māmā's wife | side `maternal`, `inLaw`, alter female |
| māsī | માસી | mother's sister | side `maternal`, blood, alter female |
| māsā | માસા | māsī's husband | side `maternal`, `inLaw`, alter male |

- **Validation**: this entire tier — the hardest tier in any Indic language — selects on
  **normalized fields only**: `{side, affinity, alterGender, apexSeniority}`. No backbone access
  needed. The `side` derivation rule (first-ascent parent's gender, never overridden by the apex)
  is exactly what keeps māmā/kākā apart, as the code comment on `derive_side` promises. ✓
- **Validation**: *moṭā bāpā* vs *kākā* compares the uncle to ego's **father**, not to ego — this
  is `apexSeniority`, the field ADR-0026 added precisely for the chāchā/tāū (Hindi) distinction.
  Gujarati confirms the need. ✓ Note usage varies by family: many use kākā for any father's
  brother; `apexSeniority = unknown` should fall back to kākā (unmarked).
- The affinal four (kākī/fuvā/māmī/māsā) are `inLaw` because the trailing `across` is not in
  ancestor position — the `derive_affinity` position rule lands them correctly. ✓
- Great-uncles/aunts (`up³·down`, removed = 2): prefix *moṭā* generalizes loosely ("moṭā māmā")
  but there are no dedicated terms → compositional/prefix fallback.

## 5. Cousins

First cousins: path `up·up·down·down`, collateral {2,2,1,0}. Gujarati treats cousins as siblings
qualified by the linking aunt/uncle; only some lines have a dedicated adjective:

| Term | Script | Gloss | Selecting keys |
| --- | --- | --- | --- |
| pitrāī bhāī / bahen | પિત્રાઈ | father's brother's child | cousinDegree 1, removed 0, side `paternal`, **⚠ backbone: first `down` hop (parent's sibling) is male** |
| masiyāī bhāī / bahen | મસિયાઈ | mother's sister's child | side `maternal`, **⚠ first `down` hop is female** |
| māmā-no dīkro/dīkrī (mamera side) | મામાનો દીકરો | mother's brother's child | side `maternal`, ⚠ first `down` hop male — usually compositional |
| foī-no dīkro/dīkrī | ફોઈનો દીકરો | father's sister's child | side `paternal`, ⚠ first `down` hop female — compositional |

- **Finding**: the four cousin lines split on `side` × **the parent's-sibling's gender** — and the
  latter is again a backbone fact (gender of the person the first `down` hop lands on). `side`
  alone distinguishes only two of the four. The parallel/cross-cousin distinction matters
  culturally (marriage eligibility), so the `gu` mapping genuinely needs this key even where the
  term is a genitive construction.
- All four collapse to plain bhāī/bahen in loose usage — a coarser `gu` entry keying only
  `{cousinDegree: 1, removed: 0}` is also legitimate. The mapping shape should allow both
  specific and coarse entries (most-specific wins).
- **Second cousins and beyond (cousinDegree ≥ 2), and removed cousins (removed ≥ 1 outside the
  uncle/nephew tier): no lexicalized Gujarati terms at all.** Usage is *dūr-nā sagā* ("distant
  kin") or a spelled-out genitive chain. This is the largest unlexicalized region → compositional
  fallback territory.

## 6. Nephews and nieces

Path `up·down·down`, collateral {1,2,0,1}:

| Term | Script | Gloss | Selecting keys |
| --- | --- | --- | --- |
| bhatrījo / bhatrījī | ભત્રીજો / ભત્રીજી | **brother's** son / daughter | alter gender, **⚠ backbone: first `down` hop (the sibling) is male** |
| bhāṇej / bhāṇejo / bhāṇī | ભાણેજ / ભાણિયો / ભાણી | **sister's** son / daughter | alter gender, **⚠ first `down` hop is female** |

- Same pattern as grandchildren and cousins: the selector is the **linking sibling's gender**, a
  backbone fact. `side` is `both` here (couple apex) and does not discriminate.
- Both terms are used by egos of either gender (a woman's brother's son is her bhatrījo too) —
  `egoGender` is *not* a selector. ✓
- Grand-nephews (`up·down·down·down`): compositional only.

## 7. Spouse and spouse-side kin

The affinal heartland. Note the classification quirk the code pins: a bare `across` path counts
zero vertical hops, so a **spouse is `classification = self` + `affinity = inLaw`**.

| Term | Script | Gloss | Path shape | Selecting keys |
| --- | --- | --- | --- | --- |
| pati / var / dhaṇī | પતિ / વર / ધણી | husband | `across` | self + `inLaw`, alter male |
| patnī / vahu | પત્ની / વહુ | wife | `across` | self + `inLaw`, alter female |
| savat / śokya | સવત / શોક્ય | co-wife | `across·across` | self + `inLaw`, alter female, **⚠ backbone: two `across` hops** |
| sasro | સસરો | spouse's father | `across·up` | lineal ancestor gen 1 + `inLaw`, alter male |
| sāsu | સાસુ | spouse's mother | `across·up` | lineal ancestor gen 1 + `inLaw`, alter female |
| jamāī | જમાઈ | daughter's husband | `down·across` | lineal descendant gen 1 + `inLaw`, alter male (properly **⚠ linking child female**) |
| vahu / putravadhū | વહુ / પુત્રવધૂ | son's wife | `down·across` | lineal descendant gen 1 + `inLaw`, alter female (properly **⚠ linking child male**) |
| sāḷo | સાળો | wife's brother | `across·up·down` | collateral {1,1,0,0} + `inLaw`, alter male, **⚠ backbone: `across` first + spouse hop is female** |
| sāḷī | સાળી | wife's sister | `across·up·down` | as sāḷo, alter female |
| jeṭh | જેઠ | husband's **elder** brother | `across·up·down` | collateral {1,1,0,0} + `inLaw`, alter male, **⚠ spouse hop male**, `apexSeniority = elder` |
| diyar / der | દિયર | husband's **younger** brother | as jeṭh, `apexSeniority = younger` |
| naṇand | નણંદ | husband's sister | `across·up·down` | ⚠ spouse hop male, alter female |
| jeṭhāṇī / derāṇī | જેઠાણી / દેરાણી | jeṭh's / diyar's wife | `across·up·down·across` | ⚠ spouse hop male, alter female, `apexSeniority` elder/younger |
| bhābhī | ભાભી | brother's wife | `up·down·across` | collateral {1,1,0,0} + `inLaw`, alter female, **⚠ `across` last + linking sibling male** |
| banevī | બનેવી | sister's husband | `up·down·across` | ⚠ `across` last + linking sibling female, alter male |
| sāḍhu (bhāī) | સાઢુ | wife's sister's husband | `across·up·down·across` | ⚠ spouse hop **female**, alter male |
| naṇdoī | નણદોઈ | husband's sister's husband | `across·up·down·across` | ⚠ spouse hop **male**, alter male |
| vevāī / vevāṇ | વેવાઈ / વેવાણ | child's spouse's father / mother | `down·across·up` | collateral {1,1,0,0} + `inLaw`, `sharing = notApplicable`, alter gender |

- **Headline finding — the normalized-field collision**: sāḷo, jeṭh/diyar, bhābhī's male
  counterpart (banevī), and vevāī are **all** `collateral {up:1, down:1, cousinDegree:0,
  removed:0}` + `inLaw`. The normalized fields cannot tell "wife's brother" from "sister's
  husband" from "co-father-in-law". What distinguishes them is **where the `across` hop sits**
  (first / last / middle) and **the gender of the person on specific hops** (spouse hop:
  jeṭh vs sāḷo; sibling hop: bhābhī vs banevī). English mostly doesn't care (all "brother-in-law"
  / "sister-in-law"), Gujarati absolutely does. The phrasing layer therefore cannot be a pure
  function of the normalized fields — it must key on backbone-derived facets.
- **Validation**: jeṭh/diyar and jeṭhāṇī/derāṇī compare the brother to ego's **husband** — the
  branch siblings at the apex — which is precisely `apexSeniority` again, now on an affinal path. ✓
- **Validation**: sasro/sāsu are the same word for husband's or wife's parents — matching the
  descriptor, where `side = notApplicable` on an `across`-initial path. ✓ Likewise jamāī/vahu key
  on alter gender in practice; keying the **linking child's** gender (backbone) instead is more
  honest once same-sex marriages are in the data (a son's husband would otherwise render jamāī —
  defensible, but it should be a deliberate `gu`-entry choice, not an accident).
- **Validation (ADR-0027)**: every lexicalized multi-marriage term found — sāḍhu, naṇdoī,
  jeṭhāṇī, derāṇī, savat — uses exactly **two** `across` hops. Nothing lexicalized in Gujarati
  (or English) exceeds the affinal traversal ceiling. ✓
- Divorce: the backbone's `across` hop carries `status = ended` and the engine deliberately still
  traverses it (report-and-tag). Whether an ended marriage still renders "sāsu" is a phrasing
  *policy*, left open here for #279/#282 — the descriptor gives phrasing everything it needs to
  decide either way.

## 8. Step and adoptive prefixes

| Term | Script | Gloss | Selecting keys |
| --- | --- | --- | --- |
| savkī mā / apar-mā | સાવકી મા / અપરમા | stepmother | lineal ancestor gen 1, affinity `step`, alter female |
| savko bāp | સાવકો બાપ | stepfather | as above, alter male |
| savkā (prefix) | સાવકા | step- (any close kin) | affinity `step` (colloquially also `sharing = half`) |
| dattak (prefix) | દત્તક | adoptive/adopted (formal register) | `edgeNature = adoptive` |

- Stepmother's path `up·across` puts the `across` in ancestor position → `affinity = step`; the
  mechanical position rule produces exactly the Gujarati category. ✓
- *dattak* is formal/legal; everyday Gujarati usually leaves adoption unmarked. An honest `gu`
  mapping can render the unmarked term for `edgeNature = adoptive` (adoption is kinship, which is
  also the toolchain's stance) and keep *dattak* available for an explicit-detail mode.
- English's independent prefixes — **step-** (`affinity`), **half-** (`sharing`), **adoptive**
  (`edgeNature`) — compose freely ("half-adoptive sibling"), which is exactly why ADR-0026 split
  the three dimensions. Gujarati merges step/half colloquially but the descriptor's split costs
  it nothing. ✓

## 9. English baseline

English is the trivial case, but it pins the *floor* of what every mapping entry keys on:

| English selector | Descriptor field(s) |
| --- | --- |
| parent/child/sibling/uncle/cousin… | `classification` (kind, generations, cousinDegree, removed) |
| grand- / great- recursion | lineal `generations` ≥ 2 (productive prefix, unbounded) |
| gendered terms (mother, nephew…) | `alterGender` (exception: *cousin* is gender-neutral) |
| -in-law suffix | `affinity = inLaw` |
| step- prefix | `affinity = step` |
| half- prefix | `sharing = half` |
| adopted/adoptive | `edgeNature = adoptive` |
| first/second cousin, once removed | materialized `cousinDegree` / `removed` (✓ ADR-0026's "don't make consumers re-derive the formulas") |

Where English **under-specifies** relative to the descriptor (all deliberate merges — many
descriptor regions → one term):

- **No side**: *grandmother* covers dādī and nānī; the optional "maternal/paternal" qualifier
  maps directly onto `side` when wanted.
- **No seniority**: neither field surfaces in any English lexeme.
- **Blood/in-law merge in aunt/uncle**: FBW (kākī) is plain "aunt" — English ignores `affinity`
  exactly where Gujarati leans on it.
- **The brother-in-law pile-up**: one English term covers sāḷo, jeṭh, diyar, banevī, sāḍhu,
  naṇdoī — six Gujarati terms across different backbone templates.
- **No co-in-law term**: vevāī/vevāṇ has no English lexeme at all ("co-father-in-law" is
  dictionary-ware) → even English needs the compositional fallback here ("your child's
  father-in-law").

Core English needs **no backbone access** — normalized fields suffice for every ordinary lexeme.
English is the degenerate case; Gujarati is the forcing one.

## 10. Unlexicalized regions — where the compositional fallback fires

Descriptor regions with **no** Gujarati lexeme (and mostly none in English):

1. **cousinDegree ≥ 2** (second cousins on) — *dūr-nā sagā* or genitive chains.
2. **removed ≥ 1** outside the parent's-sibling / sibling's-child tier.
3. **Lineal generations ≥ 4** (past par-dādā / prapautra).
4. **Grand-nephews/nieces and beyond** (`up·down·down·down`…).
5. **Almost all two-`across` chains** beyond the five lexicalized ones (wife's brother's wife has
   no stable term; "sāḷāvelī" is regional at best) — and *everything* the ADR-0027 ceiling
   excludes is a fortiori unlexicalized.
6. **`side = other` / `gender = other` regions** — no natural-language lexicon anywhere keys
   these; the fallback must handle them without pretending.
7. **`seniority`/`apexSeniority = unknown`** where a term is seniority-split (jeṭh/diyar): fall
   back to the unmarked or coordinate form, never guess.

The natural Gujarati fallback is the **genitive chain** — *māmā-no dīkro*, *dīkrī-no dīkro* —
i.e. a walk along the path backbone rendering each hop ("X's Y"). English does the same
("your mother's cousin's daughter"). **The backbone is not just an escape hatch for the phrasing
layer — it is the fallback's direct input.** A hop-by-hop renderer over `PathHop` (with per-hop
gender for dīkro/dīkrī, bhāī/bahen, pati/patnī word choice) covers every unlexicalized region in
both languages with honest output.

## Findings for the mapping-entry shape (#279)

1. **Normalized fields alone are insufficient — by a wide margin.** Four independent Gujarati
   term families key on backbone facts:
   - grandchildren via son vs daughter (pautra/dohitra) — first `down`-hop gender;
   - nephews via brother vs sister (bhatrījo/bhāṇej) — first `down`-hop gender;
   - cousin lines (pitrāī/masiyāī/…) — parent's-sibling hop gender;
   - the affinal collision class (sāḷo vs jeṭh vs banevī vs vevāī, sāḍhu vs naṇdoī, spouse vs
     savat) — `across`-hop **position** and specific hop **genders**.
   A mapping entry must be able to match on **backbone-derived facets**, not only the normalized
   dimensions.
2. **A small closed set of derived facets covers everything found.** No term needed arbitrary
   path predicates — only: (a) the **affinal template** (where `across` hops sit: none / first /
   last / first+last / ancestor-position — a handful of shapes given the ADR-0027 ceiling),
   (b) the **gender of the linking person at each named position** (spouse hop, first
   descent hop, apex branch-sibling), and (c) the two seniorities. These are all mechanically
   derivable from `path` by the consumer — **no engine change required**, consistent with
   ADR-0025's "consumers own only the UX of querying". The phrasing layer can precompute them
   into a "phrasing key" before table lookup.
3. **`apexSeniority` earns its keep three times** (moṭā bāpā/kākā, jeṭh/diyar, jeṭhāṇī/derāṇī) —
   including on *affinal* paths, which ADR-0026 anticipated. Ego-relative `seniority` is only a
   compositional modifier (moṭā/nānā bhāī). Both `unknown` values need an unmarked-term fallback
   rule in the entry shape — "honest emptiness" extended to honest terminology.
4. **The parents'-siblings tier fully validates the normalized design**: ten Gujarati terms
   select on `{side, affinity, alterGender, apexSeniority}` alone. `side`'s first-ascent rule and
   `affinity`'s position rule each produce exactly the native categories.
5. **Nothing found exceeds the descriptor.** No Gujarati or English term needs a distinction the
   descriptor + backbone cannot express, and every lexicalized term fits within the ADR-0027
   two-`across` ceiling. Conversely several descriptor dimensions are *richer* than either
   lexicon (step vs half for `gu`; side, seniority for `en`) — many-to-one collapses, which are
   safe. **No engine change is needed for the phrasing layer.**
6. **Entries need specificity ordering.** Coarse entries (cousin → bhāī/bahen) and specific ones
   (masiyāī bhāī) legitimately coexist; the lookup should take the most specific match, so the
   entry shape needs a defined precedence (e.g. by number of keyed facets).
7. **The compositional fallback is a backbone walk** (genitive chain in both languages), not a
   static string — it needs per-hop gendered noun choice. Designing the fallback *is* designing
   half the phrasing layer; unlexicalized regions (second cousins, distant affinal ties,
   `other`-gender hops) are common in real data.
8. **Policy questions surfaced but deliberately not decided here** (they belong to #279/#282):
   whether ended marriages still phrase affinal terms (`MarriageStatus` carries the fact);
   whether `edgeNature = adoptive` renders unmarked by default (recommended: yes, matching the
   toolchain's adoption-is-kinship stance) with *dattak*/adoptive available as explicit detail;
   whether jamāī/vahu key alter gender or linking-child gender under same-sex marriages.

## Sources

Term inventory verified against (accessed 2026-07-12):

- [Wikibooks — Gujarati/Family relations](https://en.wikibooks.org/wiki/Gujarati/Family_relations)
- [Learn Marathi with Kaushik — Relations in Gujarati](https://learnmarathiwithkaushik.com/courses/relations-in-gujarati/)
- [Bardai Brahmin Samaj — Know Thy Relations](https://www.bardaionline.com/religion-culture/know-thy-relations/)
- [Nanimas — Basic Gujarati Family Relations](https://nanimas.co.uk/basic-gujarati-family-relations)
- [Know Your Roots — Relationship words in Indian Languages](https://knowyourrootskyr.blogspot.com/2020/10/relationship-words-in-indian-languages.html)

Descriptor semantics from [ADR-0026](./adr/0026-relationship-descriptor-and-path-identity.md),
[ADR-0027](./adr/0027-affinal-traversal-ceiling-and-step-subsumption.md), and
[`crates/kul-core/src/query/descriptor.rs`](../crates/kul-core/src/query/descriptor.rs) (path-shape → dimension
derivations hand-checked against `derive_classification` / `derive_affinity` / `derive_side`).
