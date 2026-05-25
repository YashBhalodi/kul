# ADR 0021 — Language properties plumb through to `data-*` attributes

**Status:** Accepted
**Date:** 2026-05-25
**Deciders:** owner

## Context

A surface that wants to style or query a kinship diagram needs the *facts* behind each shape, not just its geometry. "Tint cards by gender", "fade edges from before 1950", "jump to the person under the cursor", "distinguish a current adoption from a terminated one" — each needs a Person, Marriage, or parenthood-link property to reach the emitted SVG.

Before this decision the emitter encoded a handful of facts as BEM modifier classes (`kul-card--ghost`, `kul-edge--adoption`, `kul-edge--ended`, …) and dropped the rest on the floor: `PositionedCard` carried only `person_id`, `name`, `kind`, and geometry; gender, dates, and generation never left `kul-render`. Two problems compound:

1. **The modifier-class set conflates type with property.** `kul-edge--marriage` and `kul-edge--ended` are two axes (what the edge *is* vs. whether it ended) flattened into one space-separated class list. Every new theming axis is a new class — a combinatorial seam that grows by multiplication, and one a consumer can only select on, never read a value from.
2. **There is no principle for "where does a new field surface?"** When the spec gains a field, nothing says it must reach the SVG, so it silently doesn't. Each future "style by X" request would re-litigate plumbing X through three crates.

## Decision

**Every property the language declares on a Person, Marriage, or parenthood link (birth / adoption) — and every future top-level construct — plumbs through `kul-render → kul-layout → kul-svg` and surfaces as a `data-*` attribute on the emitted element. A CSS class names only the entity *type*.**

The seam, in three conventions ([ADR-0016](./0016-visualization-pipeline-crate-boundaries.md) holds the full class list):

- **Type → class.** `kul-card`, `kul-edge` (plus the typographic helpers `kul-label-name`, `kul-ghost-badge`). The class set is small and closed; one class per primitive.
- **Property → `data-*` attribute.** Booleans as `data-is-<adjective>="true|false"` (`data-is-alive`, `data-is-ended`, `data-is-past`); enumerations as explicit strings (`data-kind`, `data-gender`, `data-ghost-reason`, `data-link-kind`, `data-end-reason`); dates in their source `~YYYY[-MM[-DD]]` form.
- **Absence omits.** A missing optional value emits no attribute at all (no empty strings) — the canonical pattern's "absence, not placeholders".

The plumb-through is exhaustive over the spec, not over a hand-picked list: `data-family`, `data-given`, and `data-adoption-end` are declared-but-rarely-themed Person / adoption fields, and they surface too. A derived fact a surface cannot reconstruct cheaply (a card's `data-generation`, an edge's `data-is-past`) is carried alongside the declared ones.

`PositionedShape` carries these as display-ready fields (dates already formatted to strings, like `PositionedCard::name`), so the emitter stays a dumb walker and `PositionedShape` gains no serialization or `kul-core` *type* surface — it pulls `ExportedDate` from its owning crate to format, per ADR-0016's "from the owning crate directly".

## Consequences

- **A new spec field has one obvious home.** When `spec/` gains a field, the responsible PR threads it through `RenderShape → PositionedShape → data-*` in the same change — the same discipline `docs/canonical-ui-pattern.md` already imposes for "how does it render". "Where does X surface?" is answered once, here.
- **Theming is additive and read-not-just-match.** A surface tints by `[data-gender]`, fades by `[data-born]`, or distinguishes `[data-is-past="true"]` adoptions — all in CSS, no Rust release. New axes add attributes, never multiply a class space.
- **Click-to-jump has its structural enabler.** `data-person-id` (and the edges' `data-child-id` / `data-marriage-id`) are the F10 hook; the handler stays chrome (ADR-0016's structural/chrome line).
- **The canonical pattern keeps its defaults.** The seam being *present* does not change what the canonical visual *does*: gender is still not card-encoded by default (`docs/canonical-ui-pattern.md`, "The uniform card") — a surface must opt in by selecting on `data-gender`.

## Anti-suggestions (do not re-propose)

- **"Re-introduce BEM modifier classes for the new axis."** That is the conflation this ADR removes. Type → class, property → `data-*`. A re-theming hook that earlier lived as a class (the removed `kul-edge--in-tree` / `--cross-tree`; see [ADR-0018](./0018-canonical-layout-algorithm.md)) comes back as a `data-*` attribute or not at all.
- **"Only plumb the fields a theme needs today."** The principle is exhaustive over the spec precisely so the next theming request needs no new plumbing. Selectively plumbing re-opens "where does X surface?" every time.
- **"Make `PositionedShape` serialize the structured property values (precision, circa as separate fields)."** The positioned shape is an internal, display-ready seam (ADR-0016). A date's precision is already encoded in its formatted value's component count; circa rides as the `~` prefix. Splitting them into structured sub-properties reifies a wire shape no consumer asked for.
- **"Emit empty `data-*=""` for missing optionals so every element has every attribute."** Absence is a value in this pattern; an empty string is a third state a consumer must special-case. Omit the attribute.
