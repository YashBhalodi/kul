# ADR 0037 — The tier-1 register's real size, the overlay stack, and the lint's reserved carve-out

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[ADR-0036](./0036-two-tier-theming-and-the-accessibility-non-goal.md) decided the two tiers, the
region model, the four reserved query hues and the structural lint.
[#305](https://github.com/YashBhalodi/kul/issues/305) built them, and building them surfaced three
questions that decision left open — each with an answer a future contributor would otherwise read as
sloppiness.

1. **"Roughly twenty values" and "renders identically" cannot both hold at face value.** The shipped
   preview bridges **21 distinct `--vscode-*` variables**, not eight. Six of them are the diagram's
   gender and edge hues, which ADR-0036 explicitly keeps. Collapsing the remaining fifteen onto eight
   colour roles re-chooses values in exactly the way "values are re-expressed, not re-chosen"
   forbids, and #305's acceptance criterion is pixel-identity in light, dark and high-contrast.
2. **"Regions" names a model, not a mechanism.** The ADR says collisions become impossible by
   construction; it does not say what the construction is, and an absolutely-positioned region whose
   children each set their own inset would satisfy the letter of it while reproducing the bug.
3. **The reserved hues have no consumer in the slice that defines them.** The lint's
   defined-but-unused rule — the rule that later forces `--kul-tooltip-*` out when #300 deletes
   `tooltip.ts` — would delete the query paint vocabulary on the day it is written.

## Decision

### Tier 1 is 38 tokens, and pixel-identity is why

The register, as shipped in [`preview-themes.css`](../../packages/preview/src/preview-themes.css):

| Group          | Count | Members                                                                                |
| -------------- | ----- | -------------------------------------------------------------------------------------- |
| Colour roles   | 11    | surface, raised, on-surface, on-raised, muted, border, shadow, accent, danger, hover, active |
| Typography     | 3     | font-diagram, font-chrome, font-size-base                                              |
| Diagram hues   | 7     | male, female, other, birth, adoption, marriage, past                                    |
| Reserved paint | 6     | sync-selection, query-selection, query-result, query-uncertain, query-path, dim-alpha    |
| Spacing scale  | 6     | space-1 … space-6 (2 / 4 / 6 / 8 / 12 / 16 px)                                           |
| Radius scale   | 2     | radius-sm, radius-md                                                                     |
| Motion         | 3     | motion-fast, motion-slow, motion-pulse                                                   |

Three roles sit beyond ADR-0036's list of eight, each because a built-in VSCode theme gives it a
value the eight cannot reproduce: **`--kul-on-raised`** (`icon.foreground` is a shade apart from
`editor.foreground` in Light+ and Dark+), and **`--kul-hover`** / **`--kul-active`** (the toolbar's
two interaction states — the eight roles name no state colour at all).

Two collapses were taken deliberately, and they are the only value changes in the rebuild. The whole
`editorHoverWidget-*` family folds onto `raised` / `on-surface` / `border`, which is a no-op under
VSCode's own colour registry — it defaults each hover-widget colour to its `editorWidget` sibling —
and diverges only for a theme that overrides the hover widget *alone*. `editorHoverWidget-statusBarBackground`,
the tooltip's separator, has no such default and genuinely shifts to `--kul-border`. Both land on
`tooltip.ts`, which #300 deletes. Separately, `--kul-card-stroke` — the un-gendered card fallback —
becomes `--kul-on-surface` instead of tracking the male hue; R03 makes gender required, so it paints
no card in the preview and no swatch in the baked legend.

The load-bearing property ADR-0036 was reaching for is **that the register does not grow per
feature**, and that survives intact: chrome growth lands in tier 2, which costs a theme nothing. A
budget assertion in the lint pins the count, so a 39th token is a deliberate act rather than a drift.

### The region model is expressed as flex flow, not as insets

The overlay region is a single bottom-left **flex column** (`column-reverse`) whose children are the
control cluster, the error popover and the legend, in that stacking order. The region owns
`position`, its inset, the gap between panels and the z-index; no chrome inside it declares any of
those. That is what makes the shipped legend/error-popover overlap *unrepresentable* rather than
merely repaired — there is no second inset left to get wrong, and a fourth panel joins by being
appended.

The overlay, float and notify regions are viewport-fixed layers rather than stage-relative boxes.
This preserves today's geometry to the pixel (the chrome has always been `position: fixed` outside
the body's padding) while leaving the flow region free to take the filter bar without moving
anything.

Three of the five regions ship empty. Declaring them empty is the point: a later slice joins one by
appending an element, not by inventing an inset. The hover tooltip moves into the float region on the
way past, since "positioned by JS against an entity's screen box" is exactly what that region means.

### The lint carves out reserved paint by name, and the carve-out is itself linted

`RESERVED_PENDING_CONSUMERS` in [`tokens.test.ts`](../../packages/preview/tests/tokens.test.ts) names
the four query hues and the filter alpha explicitly. A converse assertion fires when one of them
*gains* a consumer, so the list cannot outlive its reason.

A **section marker in the CSS** was rejected: it would let any token be smuggled past the rule by
being written in the right place, and the rule's whole value is that it is hard to satisfy.
**Consuming the hues from a placeholder rule** was rejected as a lie the lint would then bless.

## Consequences

- **A theme costs 38 values instead of ~120, and the number is asserted.** The count is roughly twice
  ADR-0036's estimate; what that ADR actually promised — a closed register that does not grow with
  the chrome — holds.
- **Two pixels move, both inside the doomed tooltip.** Its separator hairline and, for a theme that
  overrides `editorHoverWidget.*` alone, its chrome. Everything else renders identically.
- **`kul-svg`'s baked theme gains the same two tiers** and skips the chrome half of both, which is
  the same structural-subset rule it already followed. `visual.rs` asserts the skip directly instead
  of implying it.
- **Later slices place chrome by naming a region.** The panel is an entity-anchored float, the filter
  bar is in flow, toasts are transient notifications; none of them computes an inset.

## Anti-suggestions (do not re-propose)

- **"Get tier 1 down to twenty by folding `on-raised` / `hover` / `active` into the eight roles."**
  Each of the three has a distinct value in every built-in VSCode theme, so folding them re-chooses
  colours the rebuild promised to preserve. The register is closed; its exact size is not the
  property that matters.
- **"Give the reserved hues placeholder rules so the lint needs no carve-out."** A rule nothing
  renders is dead code that reads as live code, and it converts a sharp lint into a satisfied one.
- **"Position the overlay panels absolutely inside the region and skip the flex column."** That is
  the shipped bug with extra steps: two panels, two insets, one collision.
- **"Drop the three empty regions until something needs them."** Their emptiness is the interface.
  A later slice that finds no `notify` region invents an inset instead, which is precisely the
  practice ADR-0036 retired.
