# ADR 0036 — Preview theming rebuilt in two tiers, four reserved query hues, and accessibility as a stated non-goal

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[#276](https://github.com/YashBhalodi/kul/issues/276) and [#277](https://github.com/YashBhalodi/kul/issues/277)
settled what the query chrome looks like and how it behaves; [ADR-0035](./0035-detail-surface-one-selection-one-batched-lookup.md)
settled what the details panel contains. Two questions were left:
[#284](https://github.com/YashBhalodi/kul/issues/284) asks which parts of that chrome get `--kul-*`
tokens so they follow VSCode themes, and what the keyboard path is. This ADR answers both and
resolves #284 on the [query-UX wayfinder map (#275)](https://github.com/YashBhalodi/kul/issues/275).

The theming answer turned out to be much larger than "add a token family", because seven facts about
the shipped preview each moved an answer:

1. **The themed palette is fully spent.** [`preview-themes.css`](../../packages/preview/src/preview-themes.css)
   maps `--vscode-charts-blue` / `-red` / `-yellow` to the three genders and `-green` / `-orange` /
   `-purple` to birth, adoption and marriage edges. That is every hue VSCode's charts family offers.
   There is no unspent themed colour for query paint to map to.
2. **There is already one sanctioned literal.** `--kul-selected-outline-color: #ff2d95` is a fixed
   magenta, commented in place: chosen so selection "stays equally bold across light, dark, and
   high-contrast" and because it is a hue "used nowhere else in the palette".
3. **The token layer has an accidental root.** ~120 per-site tokens live under one
   `body[data-theme="vscode"]` block, on [ADR-0016](./0016-visualization-pipeline-crate-boundaries.md)'s
   deliberate rule of one token per site *even where values coincide*. Over time `--kul-control-*` —
   nominally the pan/zoom button cluster — became the de-facto base tier: `--kul-legend-bg` and
   `--kul-error-popover-bg` are both `var(--kul-control-bg)`.
4. **The positioning layer already collides.** [`legend.ts`](../../packages/preview/src/legend.ts) and
   [`errors.ts`](../../packages/preview/src/errors.ts) each own an independent visibility boolean with
   no cross-talk, and both panels pin to the identical
   `calc(var(--kul-control-inset) + var(--kul-control-size) + 14px)` bottom-left inset. Open both and
   they overlap. This is a shipped bug, found while inventorying what the new chrome would join.
5. **The tree has no accessibility surface; the chrome has a complete one.** `#root` is
   `tabindex="-1"` with `outline: none`, and no card or edge carries `tabindex`, `role` or a label.
   Meanwhile `controls.ts`, `legend.ts` and `errors.ts` carry `role`, `aria-label`, `aria-pressed`,
   `:focus-visible` and `tabindex="0"` throughout.
6. **Arrow keys are already taken, window-wide.** `mountKeyboardPan` binds the four arrows plus
   `+` / `-` / `0` on `window` with `preventDefault` and **no focus guard** — it does not check
   whether the event came from an input.
7. **The details panel is selection-scoped.** #277 rejected its variant 1 precisely because hosting
   a selection-independent filter "forced the widget to grow a no-selection state"; its resolution
   keeps the widget as "detail + Explore kin exactly as #276 decided". The panel therefore appears
   and disappears — it is not standing chrome.

## Decision

### Accessibility is a non-goal, and it is written down

The query UX assumes a user with a working mouse and keyboard. This epic builds **no** keyboard path
into a query, **no** focus machinery on the tree, **no** screen-reader affordances, and **no**
reduced-motion handling. It does not add redundant non-colour encoding for the paint vocabulary
beyond the dash patterns the design already happens to use.

The non-goal reaches into the markup: new query chrome carries **no** `role`, `aria-*` or
`:focus-visible` at all. This is deliberately unlike its neighbours (fact 5), and the inconsistency is
accepted as the honest cost of a clear position — half-built accessibility invites the assumption
that the rest exists.

This is recorded rather than left silent because the alternative reads as an oversight, and an
oversight gets "fixed" by the next contributor. It was considered against building a genuine keyboard
twin: a roving `tabindex` over the cards with focus doubling as hover so the lens fires. That is the
technically right answer and it was rejected on roadmap cost — it needs a spatial traversal order
invented over `kul-layout`, and it collides head-on with fact 6, since arrows would have to become
focus-scoped and stop meaning "pan".

**Nothing here is a licence to break what works.** The existing chrome keeps every attribute it has;
this decision governs only what is newly built.

### The token layer is rebuilt in two tiers

Every `--kul-*` name and its structure is redesigned from scratch, along with `preview.css`'s
application rules and how a theme is expressed at all. **Values are re-expressed, not re-chosen** —
the diagram keeps its gender hues, edge colours and ghost treatment, and the query chrome keeps the
visuals #276 and #277 settled. This is a theming-architecture change, not a visual redesign.

**Tier 1** is what a theme supplies: colour roles (surface / raised / on-surface / muted / border /
shadow / accent / danger), a spacing scale, a radius scale, motion durations, and the reserved
diagram and query hues. Roughly twenty values, and it stays roughly twenty as the chrome grows.
Tier 1 bridges to `--vscode-*` palette variables, so the preview keeps tracking the active VSCode
theme; only the reserved hues are literals.

**Tier 2** is the per-site semantic layer, aliasing onto tier 1 rather than onto `--vscode-*`.
ADR-0016's per-site override survives intact — a theme that wants a distinct legend border still
overrides that one alias — but it now costs a theme ~20 values instead of ~120, and the count stops
growing with every feature.

This supersedes ADR-0016's [2026-05-26 amendment](./0016-visualization-pipeline-crate-boundaries.md#amendment-2026-05-26--preview-chrome-is-a-semantic-token-layer)
on structure only. Its two load-bearing properties are preserved deliberately: each application site
still consumes exactly one `var(--kul-*)`, and the physical two-file split still holds. Fact 3 is the
reason this is a rebuild rather than an extension — the base tier already exists in practice, wearing
the name of a button cluster.

`kul-svg`'s self-contained theme bakes tier 1 plus the diagram aliases and skips the chrome aliases —
the same structural-subset rule it follows today. Renaming tokens that surface in exported SVGs is
sanctioned by ADR-0016's own line that token names are "an implementation detail of the stylesheet,
not `CONTEXT.md` domain vocabulary", so this is not treated as a breaking public change.

Two narrower options were rejected. **Component-scoped tokens**, declared on each component's root,
answer vocabulary growth best of all but dissolve the physical two-file split and leave a theme
author with no single place to read what is themeable. **A small flat set of role tokens consumed
directly**, with no per-site layer, is the most legible vocabulary of the three and gives up exactly
what ADR-0016 chose on purpose: a theme could no longer tint the legend without also tinting the
error popover and the query panel, because they would literally be the same token.

### Four query hues are reserved literals

The query paint vocabulary — selection violet, result teal, can't-say amber, resolution-path sky —
takes four tier-1 tokens with literal values, documented in place exactly as fact 2's magenta is.

Fact 1 is why: a themed query paint would necessarily land on a hue the diagram already uses for
something, so a teal result and a green birth edge, or a violet selection and a purple marriage edge,
would be the same colour in most themes. The tree would say two things at once. And fact 2's argument
applies verbatim — query paint must read as equally urgent on light, dark and high-contrast, which is
the property a palette-mapped hue cannot promise.

The cost, accepted: one layer of the preview stops tracking the VSCode theme, and the codebase goes
from one documented escape hatch to five. A hybrid — mapping the selection to `--vscode-focusBorder`,
which is what that variable means — was rejected because `--kul-jump-target-glow-color` already maps
there, so a transient "look here" pulse and a persistent "this is selected" outline would share a hue,
and it splits the vocabulary's provenance for a marginal gain.

The filter dim is a single tier-1 alpha, not a per-theme value: `opacity` composites against the
themed background, so one alpha is already theme-symmetric.

### The chrome layout becomes regions, and the panel overlays

The hand-computed `calc()` insets are replaced by an explicit region model: the stage declares
regions — the persistent filter bar in flow, the canvas, an overlay stack, entity-anchored floats,
transient notifications — and each piece of chrome belongs to one. Collisions become impossible by
construction rather than by arithmetic, which fixes fact 4's shipped legend/error-popover overlap on
the way past.

**The details panel floats over the canvas; the canvas never reflows.** ADR-0035's pan-to-centre
targets the *visible* region rather than the raw viewport, so a panel-driven walk never parks a card
behind the panel. Reading the tree is the preview's dominant use and querying is a mode you enter — a
mode should not reshape the canvas.

Fact 7 is what decides this. **Docking with reflow** was rejected because a selection-scoped panel
means the canvas resizes the moment you select and again when you clear: the tree jumps twice per
query session and `svg-pan-zoom` has to re-fit each time, since it caches viewport dimensions.
**Reserving the dock permanently** avoids both reflow and occlusion, and was rejected because it taxes
every non-querying user ~300px of tree and re-introduces the always-present empty widget #277 threw
out. The accepted cost of overlaying is that a card you were looking at can be covered the moment the
panel opens.

`mountKeyboardPan` gains a target guard so arrows and `+` / `-` / `0` do not fire from inside the
filter bar's inputs. This is not an accessibility accommodation — per fact 6 it is a plain bug the
moment that bar has text fields.

### The legend does not learn query paint

[ADR-0022](./0022-svg-legend.md) is untouched. The legend stays an explanation of *document*
vocabulary, shared verbatim with the legend `kul-svg` bakes into exported SVGs.

Query paint needs no key because it is always the direct consequence of something the user just did:
you clicked "siblings (2)" and cards lit teal, the filter bar says "7 dimmed", and #277's amber `?`
arrives with its reason whisper attached. Adding query rows would split `LEGEND_ROWS` — today one
list whose order and labels the baked SVG mirrors — into shared and preview-only halves, buying a key
for paint that is never ambiguous. The cost, accepted: a user who pans away and returns sees coloured
outlines with no key on screen.

### A structural lint holds the tiers

A Vitest suite reads both stylesheets as text and asserts the architecture: every consumed
`var(--kul-*)` is defined, no `var(--vscode-*)` appears outside tier 1, every tier-2 alias resolves to
a tier-1 primitive, no raw hex survives in the application sheet except the documented reserved hues,
and no token is defined but unused.

This catches what a 120-token rewrite actually breaks — dangling names and layer violations — and it
keeps catching them, so the two tiers cannot erode the way fact 3 shows the current scheme did. There
is direct precedent: [`visual.rs:586`](../../crates/kul-svg/tests/visual.rs) already lints the baked
token layer as text, asserting legend rules "must use `--kul-*` tokens, not hex literals". jsdom
resolves neither custom-property substitution nor layout, so a computed-value test is not reachable
with the current stack.

It does not catch "it looks wrong". **Browser-based visual regression** would, and was rejected: it
means a browser-automation dependency in a repo with none, screenshot baselines to store and re-bless
across three themes, and platform-dependent rendering flakiness sitting on the epic's critical path.
`kul-svg`'s snapshots are updated once, in one deliberate commit.

## Consequences

- **The theming rebuild is the epic's first slice.** No new query chrome can be styled before the
  architecture exists, so it sequences ahead of the panel, the filter bar and the paint vocabulary
  alike.
- **The epic gains a second Rust touchpoint.** Alongside ADR-0035's batched detail operation, the
  token rename reaches `crates/kul-svg/src/emit.rs` and the `visual.rs` assertions that name tokens
  and values literally.
- **`--kul-tooltip-*` disappears entirely**, since ADR-0035 deletes `tooltip.ts`. The rewrite is the
  natural moment to drop it rather than rename it.
- **A shipped bug is fixed as a side effect.** The legend and the error popover stop overlapping,
  which no ticket had noticed and no ticket owned.
- **ADR-0016's token amendment is superseded on structure, preserved on principle.** One
  `var(--kul-*)` per site and the physical two-file split both survive; what changes is where the
  values come from and how many a theme must supply.
- **The preview stops fully tracking the VSCode theme in one layer.** Five literal hues now sit
  outside the palette bridge — the four query paints plus the existing selection magenta.
- **This is the project's first written position on accessibility.** It is a non-goal, not an
  omission, and future work that wants to change it should supersede this ADR rather than treat the
  gap as a bug.

## Anti-suggestions (do not re-propose)

- **"Add a keyboard path — roving `tabindex` on the cards, focus doubling as hover."** Considered in
  full and rejected on roadmap cost, not on merit. It needs a spatial traversal order invented over
  `kul-layout`, and arrows would have to become focus-scoped, changing what they have always meant.
  The position is deliberate; reversing it is a new decision, not a bug fix.
- **"At least add `aria-label` to the new chrome — it's one attribute."** Rejected on purpose. Partial
  accessibility is worse than none, because it advertises a path that dead-ends at the tree, where
  every query actually begins.
- **"Map the query hues to `--vscode-charts-*` so the preview tracks the theme everywhere."** The
  charts family is fully spent on document meaning (fact 1), so every mapping collides with something
  the diagram already says. Dash pattern and outline offset do not carry that weight alone.
- **"Keep the ~120 per-site tokens and just add a `--kul-query-*` family."** That was the bounded
  option and it was rejected: it leaves `--kul-control-*` as the accidental base tier, leaves the
  legend/error overlap in place, and positions a docked panel by absolute inset against a canvas that
  does not know it shrank.
- **"Redesign the diagram's visual language while the tokens are being rewritten."** Out of scope by
  decision. It crosses into what `kul-svg` bakes, putting `emit.rs` and the `visual.rs` snapshot suite
  on the epic's critical path for a change nobody asked for.
- **"Reconsider the query visuals from first principles now that theming is open."** #276 and #277 are
  closed after twenty prototype rounds, with resolution comments and living prototype branches behind
  them. Re-opening them inside the epic that was meant to build them is the definition of churn.
- **"Dock the panel so nothing is ever hidden."** The panel is selection-scoped (fact 7), so docking
  costs a canvas reflow and a `svg-pan-zoom` re-fit on every select and every clear. Occlusion-aware
  centring buys the same honesty for the case that matters.
- **"Give the query paints legend rows so the tree is self-explaining."** Query paint is always caused
  by a visible action that explains it, and the legend is shared verbatim with the baked SVG, which
  will never contain query paint.
- **"Add screenshot regression so the layout rebuild is verified."** Rejected on dependency weight and
  CI flakiness. The structural lint covers the failure mode a rename actually has; the layout is
  reviewed once, deliberately.
