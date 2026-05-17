# Translation playbook

Five paired NL↔.kul examples showing capabilities composed against real prose.

## Capabilities to lean on when prose is incomplete

Kul's surface gives you several knobs for handling under-specified prose. Awareness of these is what the playbook teaches; how to deploy them on any given paragraph is the author's call.

- **Absent fields are valid** (except `name`/`gender` on `person` and `start` on `marriage`). There is no `unknown` literal; absence is the canonical "not recorded" signal.
- **Date granularity** can be a full date, year-month, year, or `~`-prefixed circa (±5y tolerance). Match the prose.
- **Comments** are preserved verbatim by the formatter and can sit on any line or trailing any statement. They're the place to record provenance, inferences, or anything the language doesn't model.
- **Parenthood requires a marriage.** A `birth` sub-statement points at a marriage id; if prose names parents without a marriage, the marriage needs to be declared too for the bio link to exist.
- **Derived relations resolve to primitives.** Uncle, cousin, half-sibling, step-parent etc. aren't keywords — they fall out of the person + marriage + birth/adoption graph (see [`vocabulary.md`](./vocabulary.md)).

## Example 1 — Simple nuclear family

Mirrors [`examples/02-nuclear-family/`](../../../examples/02-nuclear-family/nuclear-family.kul).

> Alice Sharma was born on April 12, 1950. Her husband Bob Sharma was born on November 30, 1948. They married on May 12, 1972. They had one daughter, Carol Sharma, born September 3, 1975.

```
person alice  name:"Alice Sharma"  gender:female  born:1950-04-12
person bob    name:"Bob Sharma"    gender:male    born:1948-11-30
person carol  name:"Carol Sharma"  gender:female  born:1975-09-03
  birth m_alice_bob

marriage m_alice_bob alice bob  start:1972-05-12
```

"Daughter" → `birth m_alice_bob`. Marriage id follows `m_<a>_<b>`.

## Example 2 — Three generations with derived relations, divorce, adoption, circa date

Mirrors [`examples/03-three-generations/`](../../../examples/03-three-generations/three-generations.kul).

> Ramesh and Sita Sharma were Alice's parents. Ramesh was born March 10, 1925; Sita July 15, 1928. They married June 10, 1948. Ramesh died August 22, 2005; Sita November 4, 2010.
>
> Their daughter Alice (born April 12, 1950) married Bob Sharma (born November 30, 1948) on May 12, 1972. The marriage ended in divorce on August 1, 1990. Bob died March 15, 2020.
>
> Alice and Bob had a daughter Carol, born September 3, 1975. They also adopted a son, Ravi, in June 1985 — Ravi himself had been born around 1980, exact date unknown.

```
person ramesh  name:"Ramesh Sharma"  gender:male    born:1925-03-10  died:2005-08-22
person sita    name:"Sita Sharma"    gender:female  born:1928-07-15  died:2010-11-04

marriage m_ramesh_sita ramesh sita  start:1948-06-10

person alice  name:"Alice Sharma"  gender:female  born:1950-04-12
  birth m_ramesh_sita
person bob    name:"Bob Sharma"    gender:male    born:1948-11-30  died:2020-03-15

marriage m_alice_bob alice bob  start:1972-05-12  end:1990-08-01  end_reason:divorce

person carol  name:"Carol Sharma"  gender:female  born:1975-09-03
  birth m_alice_bob
person ravi   name:"Ravi Sharma"   gender:male    born:~1980
  adoption m_alice_bob  start:1985-06-01
```

"Daughter" → `birth`. Divorce → `end:` + `end_reason:divorce`. "Around 1980" → `~1980`.

## Example 3 — Implicit marriage

> John and Mary had three children: Tom (born 1962), Susan (born 1965), and Peter (born 1968). The narrative doesn't say when John and Mary were married. John died in 1990; Mary died in 2005.

```
person john   name:"John"   gender:male    died:1990  # gender inferred from name
person mary   name:"Mary"   gender:female  died:2005  # gender inferred from name

# Marriage not stated in source; implied by shared parenthood of Tom/Susan/Peter.
marriage m_john_mary john mary  start:~1962

person tom    name:"Tom"    gender:male    born:1962
  birth m_john_mary
person susan  name:"Susan"  gender:female  born:1965
  birth m_john_mary
person peter  name:"Peter"  gender:male    born:1968
  birth m_john_mary
```

Marriage `start:~1962` defended by Tom's birth year ± tolerance. Comments flag the inferences.

## Example 4 — Missing / circa dates, conflicting account

> Our grandfather was born sometime in the mid-1920s — Aunt Priya says 1925, the family bible says 1923. He married our grandmother around 1948; the exact wedding date isn't recorded. She died in March 1995; he died October 12, 2010. They had a son born September 1950 (month only) and a daughter whose birth year nobody remembers.

```
person grandfather  name:"Grandfather"  gender:male    born:~1925  died:2010-10-12  # bible says 1923
person grandmother  name:"Grandmother"  gender:female              died:1995-03

marriage m_grandparents grandfather grandmother  start:~1948  # exact date not recorded

person son       name:"Son"       gender:male    born:1950-09  # day not recorded
  birth m_grandparents
person daughter  name:"Daughter"  gender:female                # birth year unknown
  birth m_grandparents
```

Granularity tracks the prose. Daughter's `born` is omitted (no `unknown` literal). The 1923/1925 conflict is in a `#` comment.

## Example 5 — Cross-file split

Mirrors [`examples/07-multi-file-extended-family/`](../../../examples/07-multi-file-extended-family/).

> Ramesh and Sita Patel were the founders. Ramesh: born April 10, 1928, died December 3, 2010. Sita: born September 22, 1931, died June 14, 2018. They married February 18, 1952.
>
> Their two children: Alice (b. July 19, 1955) and Dev (b. March 28, 1958). Alice married Bob Khan (b. November 2, 1953) on September 15, 1978. Dev married Eva Singh (b. December 4, 1959) on April 22, 1982.
>
> Alice and Bob had two daughters, Carol (b. June 8, 1981) and Diya (b. October 30, 1984). Dev and Eva had Farid (b. January 17, 1985) and Gita (b. approximately 1988).

`kul.yml`:
```yaml
kul: "0.1"
```

`01-founders.kul`:
```
person ramesh  name:"Ramesh Patel"  gender:male    born:1928-04-10  died:2010-12-03
person sita    name:"Sita Patel"    gender:female  born:1931-09-22  died:2018-06-14

marriage m_ramesh_sita ramesh sita  start:1952-02-18
```

`02-parents.kul`:
```
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
person carol  name:"Carol Patel"  gender:female  born:1981-06-08
  birth m_alice_bob
person diya   name:"Diya Patel"   gender:female  born:1984-10-30
  birth m_alice_bob

person farid  name:"Farid Patel"  gender:male    born:1985-01-17
  birth m_dev_eva
person gita   name:"Gita Patel"   gender:female  born:~1988
  birth m_dev_eva
```

Cross-file `birth` references (`m_ramesh_sita`, `m_alice_bob`, `m_dev_eva`) resolve by bare id — one logical namespace per directory. The split shown above is one of many the author could choose; the language imposes nothing beyond "one `kul.yml` + N `.kul` files in one directory."
