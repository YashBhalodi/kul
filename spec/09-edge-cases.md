# 9. Edge cases

Short snippets that demonstrate corner cases the language handles cleanly.

## 9.1 Founder persons

A person without a `birth` sub-statement is implicitly a documentation root. No keyword is needed:

```
person grandfather name:"Grandfather" gender:male
```

If parents are later learned and added, the existing person line need not change — only a `birth` sub-statement is appended.

## 9.2 Adoption-only persons

A person with an `adoption` sub-statement but no `birth` sub-statement is documented only by their adoptive lineage:

```
person foundling name:"Anika" born:~1985 gender:female
  adoption m_adoptive_couple start:1986-04-01
```

## 9.3 Same-pair remarriage

Two distinct marriages between the same pair of persons receive distinct IDs:

```
marriage m_alice_bob_1 alice bob start:1972-05-12 end:1980-01-01 end_reason:divorce
marriage m_alice_bob_2 alice bob start:1985-06-15
```

A child of either marriage references the appropriate marriage ID via their `birth` sub-statement.

## 9.4 Circa dates

A circa-prefixed date denotes "approximately this date, with imprecision beyond the literal's granularity":

```
person grandfather born:~1925         # somewhere in the mid-1920s
marriage m_g grandfather x start:~1948  # married around 1948
```

Validators apply a ±5-year tolerance to circa dates when comparing.

## 9.5 Marriages ended only by spousal death

A marriage in which one spouse has died but no formal end was recorded simply has no `end` field:

```
person bob   name:"Bob"   died:2020-03-15 gender:male
person alice name:"Alice" gender:female

marriage m_alice_bob alice bob start:1972-05-12
```

The marriage is no longer active at any time after Bob's death (per [Section 6.2 — Active marriage at time T](./06-semantics.md#62-active-marriage-at-time-t)), but its record is unchanged.

## 9.6 A marriage ended on a known-but-vague date

If you know a marriage ended approximately in 1990 but not the exact date:

```
marriage m_x alice bob start:1972 end:~1990 end_reason:divorce
```

## 9.7 Multiple adoptions

A person may have more than one adoption event, possibly with one ended:

```
person someone name:"Someone" born:1985-01-01 gender:female
  adoption m_first_couple  start:1985-06-01 end:1990-01-01
  adoption m_second_couple start:1992-04-15
```

The first adoption ended in 1990; the second is ongoing.

## 9.8 Bio + adoptive parents coexisting

A person may have biological parents documented AND be adopted by another couple:

```
person ravi name:"Ravi Sharma" born:1980-02-14 gender:male
  birth m_birth_parents
  adoption m_alice_bob start:1985-06-01
```

Both relationships coexist; neither replaces the other.

---

← [Section 8 — Worked examples](./08-worked-examples.md) | [Index](./README.md) | Next → [Section 10 — File conventions](./10-file-conventions.md)
