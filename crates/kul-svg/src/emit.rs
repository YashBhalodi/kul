//! SVG string templating from `PositionedShape`.
//!
//! Stateless. Walks the shape once and writes into a single `String`
//! buffer. Uses semantic CSS classes only — no inline colours, no
//! script.

use std::fmt::Write;

use kul_layout::{EdgeKind, PositionedCard, PositionedEdge, PositionedShape, SlotKind};
use kul_render::GhostReason;

/// Theme / emission configuration.
///
/// Forward-compatibility seam (per [ADR-0016](../../docs/adr/0016-visualization-pipeline-crate-boundaries.md)):
/// the output stays theme-agnostic by default, and opt-in fields tune
/// emission without changing [`crate::render`]'s signature. The private
/// trailing field keeps construction additive — build with
/// `ThemeConfig { self_contained: true, ..Default::default() }`.
// A private unit field — not `#[non_exhaustive]` — reserves room for
// future additive fields. Cross-crate construction goes through
// [`ThemeConfig::with_self_contained`] / [`ThemeConfig::with_legend`]
// / [`ThemeConfig::default`] rather than a struct literal, so a new
// field never breaks a caller.
#[allow(clippy::manual_non_exhaustive)]
#[derive(Debug, Clone, Default)]
pub struct ThemeConfig {
    /// Bake a concrete neutral light theme into the SVG as an inline
    /// `<style>` (the first child of the root `<svg>`), making the file
    /// self-contained: it renders correctly opened in any browser,
    /// dropped into an `<img>`, or embedded in a static page with no
    /// external CSS. Default `false` keeps the output theme-agnostic
    /// (no inline colours) per ADR-0016; only `kul export --format=svg`
    /// opts in. The baked stylesheet reuses the `--kul-*` token
    /// vocabulary (a subset of the VSCode preview's structural rules)
    /// and excludes all surface chrome — pan/zoom, hover, selection, and
    /// the ghost `↺` badge. Read directly; construct via
    /// [`ThemeConfig::with_self_contained`].
    pub self_contained: bool,
    /// Emit a legend (the canonical-pattern visual key) in a reserved
    /// band at the bottom-left of the diagram. The legend is built from
    /// the [`PositionedShape`] — rows appear only for categories the
    /// diagram actually surfaces (ADR-0022). The legend's *colour*
    /// resolves through the surrounding stylesheet (no hardcoded swatch
    /// colours are emitted), so it is only meaningful when a matching
    /// stylesheet is present — typically together with
    /// [`self_contained`](ThemeConfig::self_contained), which bakes
    /// the small `.kul-legend …` size-override block alongside the
    /// structural rules. Default `false` keeps the always-emitted
    /// (preview / wasm) SVG byte-unchanged and ships no English on
    /// that path. Construct via [`ThemeConfig::with_legend`].
    pub legend: bool,
    _private: (),
}

impl ThemeConfig {
    /// Build a config with [`self_contained`](ThemeConfig::self_contained)
    /// set. The construction seam for consumers outside this crate (the
    /// private field blocks a struct literal), keeping new fields purely
    /// additive.
    pub fn with_self_contained(self_contained: bool) -> Self {
        Self {
            self_contained,
            ..Default::default()
        }
    }

    /// Chainable setter for [`legend`](ThemeConfig::legend). Composes
    /// with [`with_self_contained`](ThemeConfig::with_self_contained) —
    /// `ThemeConfig::with_self_contained(true).with_legend(true)` is
    /// the CLI export's combined opt-in (ADR-0022).
    pub fn with_legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }
}

pub(crate) fn render(positioned: &PositionedShape, config: &ThemeConfig) -> String {
    let mut out = String::with_capacity(2048);
    // The legend (ADR-0022) is an emission concern, not a layout
    // concern: the positioned shape is unchanged and the legend is
    // appended into a reserved bottom-left band that grows the viewBox
    // by exactly its own height + gap, never overlapping the diagram.
    let rows = if config.legend {
        legend_rows(positioned)
    } else {
        Vec::new()
    };
    let legend_extra_height = if rows.is_empty() {
        0.0
    } else {
        LEGEND_GAP + (rows.len() as f64) * LEGEND_ROW_HEIGHT + LEGEND_BOTTOM_PAD
    };
    // The diagram is almost always wider than the legend; max() guards
    // the degenerate two-card-no-edges case where the legend overflows.
    let canvas_width = if rows.is_empty() {
        positioned.width
    } else {
        positioned.width.max(LEGEND_TOTAL_WIDTH)
    };
    let canvas_height = positioned.height + legend_extra_height;
    write_open(&mut out, canvas_width, canvas_height);
    // The inline stylesheet, when opted in, is the first child of the
    // root `<svg>` so its `svg`-scoped tokens are in scope for every
    // element below.
    if config.self_contained {
        out.push_str(SELF_CONTAINED_STYLE);
    }
    for edge in &positioned.edges {
        write_edge(&mut out, edge);
    }
    for card in &positioned.cards {
        write_card(&mut out, card);
    }
    if !rows.is_empty() {
        write_legend(&mut out, &rows, positioned.height + LEGEND_GAP);
    }
    out.push_str("</svg>");
    out
}

/// Concrete neutral light theme baked into a self-contained SVG
/// ([`ThemeConfig::self_contained`]). The token table fixes one default
/// palette in hex; the application rules below are the *structural*
/// subset of `editor/vscode/media/preview.css` — card fill/stroke, the
/// per-gender tint, the ghost translucency, edge colours per link kind,
/// and the ended-marriage fade. Everything chrome is excluded: pan/zoom
/// controls, the error banner, `:hover`, selection sync, and the ghost
/// `↺` badge (an exported ghost shows its dashed border + translucent
/// fill and no badge, per ADR-0016). The structural dasharrays ship
/// inline from the emitter and need no CSS.
const SELF_CONTAINED_STYLE: &str = r#"<style>
svg {
  --kul-preview-bg: #ffffff;
  --kul-font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  --kul-card-fill: #ffffff;
  --kul-card-stroke: #455a64;
  --kul-card-stroke-male: #1565c0;
  --kul-card-stroke-female: #c2185b;
  --kul-card-stroke-other: #f9a825;
  --kul-ghost-fill: #eceff1;
  --kul-ghost-stroke: #90a4ae;
  --kul-ghost-stroke-male: #1565c0;
  --kul-ghost-stroke-female: #c2185b;
  --kul-ghost-stroke-other: #f9a825;
  --kul-label-fill: #1a1a1a;
  --kul-ghost-label-fill: #607d8b;
  --kul-edge-stroke: #2e7d32;
  --kul-adoption-edge-stroke: #ef6c00;
  --kul-marriage-edge-stroke: #6a1b9a;
  --kul-card-stroke-width: 1.5;
  --kul-edge-stroke-width: 1.5;
  --kul-marriage-edge-stroke-width: 8.75;
  --kul-ghost-fill-opacity: 0.5;
  --kul-ended-edge-stroke-opacity: 0.6;
  --kul-label-font-size: 13px;
  --kul-legend-label-font-size: 12px;
  --kul-legend-marriage-edge-stroke-width: 5;
  background-color: var(--kul-preview-bg);
  font-family: var(--kul-font-family);
}
.kul-card rect { fill: var(--kul-card-fill); stroke: var(--kul-card-stroke); stroke-width: var(--kul-card-stroke-width); }
.kul-card[data-gender="male"] rect { stroke: var(--kul-card-stroke-male); }
.kul-card[data-gender="female"] rect { stroke: var(--kul-card-stroke-female); }
.kul-card[data-gender="other"] rect { stroke: var(--kul-card-stroke-other); }
.kul-card[data-kind="ghost"] rect { fill: var(--kul-ghost-fill); stroke: var(--kul-ghost-stroke); fill-opacity: var(--kul-ghost-fill-opacity); }
.kul-card[data-kind="ghost"][data-gender="male"] rect { stroke: var(--kul-ghost-stroke-male); }
.kul-card[data-kind="ghost"][data-gender="female"] rect { stroke: var(--kul-ghost-stroke-female); }
.kul-card[data-kind="ghost"][data-gender="other"] rect { stroke: var(--kul-ghost-stroke-other); }
.kul-label-name { fill: var(--kul-label-fill); font-size: var(--kul-label-font-size); }
.kul-card[data-kind="ghost"] .kul-label-name { fill: var(--kul-ghost-label-fill); }
.kul-edge { stroke: var(--kul-edge-stroke); stroke-width: var(--kul-edge-stroke-width); }
.kul-edge[data-link-kind="adoption"] { stroke: var(--kul-adoption-edge-stroke); }
.kul-edge[data-link-kind="marriage"] { stroke: var(--kul-marriage-edge-stroke); stroke-width: var(--kul-marriage-edge-stroke-width); }
.kul-edge[data-is-ended="true"] { stroke-opacity: var(--kul-ended-edge-stroke-opacity); }
.kul-legend-label { fill: var(--kul-label-fill); font-size: var(--kul-legend-label-font-size); }
.kul-legend .kul-edge[data-link-kind="marriage"] { stroke-width: var(--kul-legend-marriage-edge-stroke-width); }
</style>"#;

fn write_open(out: &mut String, width: f64, height: f64) {
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">"#,
        w = fmt_num(width),
        h = fmt_num(height),
    );
}

fn write_card(out: &mut String, card: &PositionedCard) {
    // Entity class names the type only; every property is a `data-*`
    // attribute (ADR-0016 class vocabulary, ADR-0021 plumb-through).
    let (kind, ghost_reason) = match card.kind {
        SlotKind::Canonical => ("canonical", None),
        SlotKind::Ghost { reason } => {
            let reason = match reason {
                GhostReason::PastMarriage => "past-marriage",
                GhostReason::PastAdoption => "past-adoption",
                GhostReason::PastBirth => "past-birth",
            };
            ("ghost", Some(reason))
        }
    };
    let _ = write!(
        out,
        r#"<g class="kul-card" data-person-id="{id}" data-kind="{kind}""#,
        id = escape_xml(&card.person_id),
    );
    if let Some(reason) = ghost_reason {
        let _ = write!(out, r#" data-ghost-reason="{reason}""#);
    }
    // Boolean properties use `data-is-<adjective>`; a person is alive
    // iff no `died:` is recorded (death lives on the person).
    let _ = write!(
        out,
        r#" data-gender="{gender}" data-is-alive="{alive}""#,
        gender = card.gender,
        alive = card.died.is_none(),
    );
    // Missing optional values omit the attribute entirely (no empty
    // strings) — the canonical pattern's "absence, not placeholders".
    write_opt_attr(out, "data-born", card.born.as_deref());
    write_opt_attr(out, "data-died", card.died.as_deref());
    write_opt_attr(out, "data-family", card.family.as_deref());
    write_opt_attr(out, "data-given", card.given.as_deref());
    let _ = write!(out, r#" data-generation="{}">"#, card.generation);
    // Ghost cards ship with stroke-dasharray inline (structural, per
    // the uniform card — see ADR-0016 §"the structural/chrome line").
    let dash = if matches!(card.kind, SlotKind::Ghost { .. }) {
        r#" stroke-dasharray="3 2""#
    } else {
        ""
    };
    // Soft corner radius — pure visual polish, consumers can override
    // via CSS (`rect { rx: 0 }`) for a sharper look.
    let _ = write!(
        out,
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" ry="{r}"{dash}/>"#,
        x = fmt_num(card.x),
        y = fmt_num(card.y),
        w = fmt_num(card.width),
        h = fmt_num(card.height),
        r = fmt_num(CARD_CORNER_RADIUS),
    );
    let label_x = card.x + card.width / 2.0;
    let label_y = card.y + card.height / 2.0;
    let _ = write!(
        out,
        r#"<text class="kul-label-name" x="{x}" y="{y}" text-anchor="middle" dominant-baseline="central">{name}</text>"#,
        x = fmt_num(label_x),
        y = fmt_num(label_y),
        name = escape_xml(&card.name),
    );
    out.push_str("</g>");
}

fn write_edge(out: &mut String, edge: &PositionedEdge) {
    // Entity class names the type only (`kul-edge`); the link kind and
    // every other property are `data-*` attributes (ADR-0016 class
    // vocabulary, ADR-0021 plumb-through). The marriage id is common to
    // every edge kind.
    let _ = write!(
        out,
        r#"<path class="kul-edge" data-marriage-id="{mid}""#,
        mid = escape_xml(&edge.marriage_id),
    );
    // Adoption edges ship with stroke-dasharray inline (structural, per
    // edges encode link kind — see ADR-0016 §"the structural/chrome
    // line"). Birth edges are solid; marriage edges (ADR-0020) are solid
    // and thick, the weight set by the consuming stylesheet against
    // `data-link-kind="marriage"`.
    let mut dash = "";
    match &edge.kind {
        EdgeKind::Birth { child_id, is_past } => {
            let _ = write!(
                out,
                r#" data-link-kind="birth" data-child-id="{cid}" data-is-past="{past}""#,
                cid = escape_xml(child_id),
                past = is_past,
            );
        }
        EdgeKind::Adoption {
            child_id,
            is_past,
            start,
            end,
        } => {
            let _ = write!(
                out,
                r#" data-link-kind="adoption" data-child-id="{cid}" data-is-past="{past}""#,
                cid = escape_xml(child_id),
                past = is_past,
            );
            write_opt_attr(out, "data-adoption-start", start.as_deref());
            write_opt_attr(out, "data-adoption-end", end.as_deref());
            dash = r#" stroke-dasharray="6 4""#;
        }
        EdgeKind::Marriage {
            host_id,
            joining_id,
            start,
            end,
            end_reason,
            is_ended,
        } => {
            let _ = write!(
                out,
                r#" data-link-kind="marriage" data-host-id="{host}" data-joining-id="{joining}" data-start="{start}" data-is-ended="{ended}""#,
                host = escape_xml(host_id),
                joining = escape_xml(joining_id),
                start = escape_xml(start),
                ended = is_ended,
            );
            write_opt_attr(out, "data-end", end.as_deref());
            write_opt_attr(out, "data-end-reason", end_reason.as_deref());
        }
    }
    let d = polyline_to_rounded_path(&edge.points, EDGE_CORNER_RADIUS);
    let _ = write!(out, r#" fill="none" d="{d}"{dash}/>"#);
}

/// Write ` name="value"` (XML-escaped) when `value` is `Some`; emit
/// nothing when `None`. The canonical pattern's "absence, not
/// placeholders": a missing optional property omits the attribute
/// entirely rather than emitting an empty string.
fn write_opt_attr(out: &mut String, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        let _ = write!(out, r#" {name}="{}""#, escape_xml(value));
    }
}

/// Card-corner radius in pixels. Visual polish — surface stylesheets
/// can override via CSS (`rect { rx: 0 }`).
const CARD_CORNER_RADIUS: f64 = 8.0;

/// Edge-corner radius in pixels. The polyline's 90° bends become
/// quadratic-Bézier arcs of this radius; consumers wanting hard
/// corners can target the `kul-edge` class and re-emit, but the
/// default canonical visual is soft.
const EDGE_CORNER_RADIUS: f64 = 10.0;

// -- Legend (ADR-0022) -----------------------------------------------
//
// The legend is appended into a reserved bottom-left band when
// `ThemeConfig::legend` is set. Each row is a swatch + a label; the
// swatch is a miniature of the real glyph, carrying the production
// `kul-card` / `kul-edge` class plus the same `data-*` attributes —
// so the surrounding stylesheet themes it for free. Only size /
// stroke-width / dash are tuned for swatch scale via the small
// `.kul-legend …` block in `SELF_CONTAINED_STYLE`; colour is never
// overridden there.

/// Vertical gap between the diagram's bottom edge and the first
/// legend row, *and* the row pitch's structural anchor.
const LEGEND_GAP: f64 = 16.0;
/// Height of one legend row — sized so the production 8.75px marriage
/// stroke (clamped to 5px via `.kul-legend` override) still reads as a
/// distinct thick line in the row, not the whole row.
const LEGEND_ROW_HEIGHT: f64 = 22.0;
/// Extra space below the last row before the viewBox bottom edge.
const LEGEND_BOTTOM_PAD: f64 = 8.0;
/// X inset of the swatch column from the canvas's left edge.
const LEGEND_LEFT_PAD: f64 = 16.0;
/// Horizontal swatch span (rect width, edge segment length).
const LEGEND_SWATCH_WIDTH: f64 = 30.0;
/// Vertical swatch span for the rect (card) swatches; edge swatches
/// are single horizontal lines through the row's centre.
const LEGEND_SWATCH_HEIGHT: f64 = 14.0;
/// Gap between swatch and its label.
const LEGEND_LABEL_GAP: f64 = 10.0;
/// Conservative label-column budget — labels are short English words,
/// not measured, so this only matters for the degenerate "diagram
/// narrower than the legend" case where the viewBox max()es up to fit.
const LEGEND_LABEL_BUDGET: f64 = 130.0;
/// Total horizontal footprint a legend occupies in the canvas.
const LEGEND_TOTAL_WIDTH: f64 = LEGEND_LEFT_PAD
    + LEGEND_SWATCH_WIDTH
    + LEGEND_LABEL_GAP
    + LEGEND_LABEL_BUDGET
    + LEGEND_LEFT_PAD;

/// The canonical legend categories, in their normative order
/// (`docs/canonical-ui-pattern.md`). Each is dynamic: a row appears
/// only when the diagram contains at least one element of that
/// category — no adoption edges → no adoption row.
#[derive(Clone, Copy)]
enum LegendRow {
    GenderMale,
    GenderFemale,
    GenderOther,
    PastRecord,
    Birth,
    Adoption,
    Marriage,
    EndedMarriage,
}

impl LegendRow {
    /// The English label string for this row (hardcoded inside the
    /// emitter; the opt-in `legend=true` path is the only consumer, so
    /// no English ships on the always-emitted SVG — see ADR-0022).
    fn label(self) -> &'static str {
        match self {
            LegendRow::GenderMale => "Male",
            LegendRow::GenderFemale => "Female",
            LegendRow::GenderOther => "Other",
            LegendRow::PastRecord => "Past record",
            LegendRow::Birth => "Birth",
            LegendRow::Adoption => "Adoption",
            LegendRow::Marriage => "Marriage",
            LegendRow::EndedMarriage => "Ended marriage",
        }
    }
}

/// Walk the [`PositionedShape`] and emit the rows whose category is
/// present, in canonical order.
fn legend_rows(shape: &PositionedShape) -> Vec<LegendRow> {
    let mut rows = Vec::new();
    let has_gender = |g: &str| shape.cards.iter().any(|c| c.gender == g);
    if has_gender("male") {
        rows.push(LegendRow::GenderMale);
    }
    if has_gender("female") {
        rows.push(LegendRow::GenderFemale);
    }
    if has_gender("other") {
        rows.push(LegendRow::GenderOther);
    }
    if shape
        .cards
        .iter()
        .any(|c| matches!(c.kind, SlotKind::Ghost { .. }))
    {
        rows.push(LegendRow::PastRecord);
    }
    if shape
        .edges
        .iter()
        .any(|e| matches!(e.kind, EdgeKind::Birth { .. }))
    {
        rows.push(LegendRow::Birth);
    }
    if shape
        .edges
        .iter()
        .any(|e| matches!(e.kind, EdgeKind::Adoption { .. }))
    {
        rows.push(LegendRow::Adoption);
    }
    if shape.edges.iter().any(|e| {
        matches!(
            &e.kind,
            EdgeKind::Marriage {
                is_ended: false,
                ..
            }
        )
    }) {
        rows.push(LegendRow::Marriage);
    }
    if shape
        .edges
        .iter()
        .any(|e| matches!(&e.kind, EdgeKind::Marriage { is_ended: true, .. }))
    {
        rows.push(LegendRow::EndedMarriage);
    }
    rows
}

fn write_legend(out: &mut String, rows: &[LegendRow], legend_top: f64) {
    out.push_str(r#"<g class="kul-legend">"#);
    for (i, row) in rows.iter().enumerate() {
        let row_top = legend_top + (i as f64) * LEGEND_ROW_HEIGHT;
        let center_y = row_top + LEGEND_ROW_HEIGHT / 2.0;
        let swatch_top = center_y - LEGEND_SWATCH_HEIGHT / 2.0;
        write_legend_swatch(out, *row, LEGEND_LEFT_PAD, swatch_top, center_y);
        let _ = write!(
            out,
            r#"<text class="kul-legend-label" x="{x}" y="{y}" dominant-baseline="central">{label}</text>"#,
            x = fmt_num(LEGEND_LEFT_PAD + LEGEND_SWATCH_WIDTH + LEGEND_LABEL_GAP),
            y = fmt_num(center_y),
            label = escape_xml(row.label()),
        );
    }
    out.push_str("</g>");
}

fn write_legend_swatch(out: &mut String, row: LegendRow, x: f64, swatch_top: f64, center_y: f64) {
    match row {
        LegendRow::GenderMale | LegendRow::GenderFemale | LegendRow::GenderOther => {
            let gender = match row {
                LegendRow::GenderMale => "male",
                LegendRow::GenderFemale => "female",
                LegendRow::GenderOther => "other",
                _ => unreachable!(),
            };
            // Carry data-kind="canonical" + data-gender so the production
            // `.kul-card[data-gender="…"] rect` rule themes the stroke
            // automatically — no hardcoded swatch colour.
            let _ = write!(
                out,
                r#"<g class="kul-card" data-kind="canonical" data-gender="{gender}"><rect x="{x}" y="{y}" width="{w}" height="{h}" rx="3" ry="3"/></g>"#,
                x = fmt_num(x),
                y = fmt_num(swatch_top),
                w = fmt_num(LEGEND_SWATCH_WIDTH),
                h = fmt_num(LEGEND_SWATCH_HEIGHT),
            );
        }
        LegendRow::PastRecord => {
            // Ghost card: `data-kind="ghost"` triggers the production
            // faded-fill rule; the dashed border ships inline (structural,
            // ADR-0016) exactly as on a real ghost card.
            let _ = write!(
                out,
                r#"<g class="kul-card" data-kind="ghost"><rect x="{x}" y="{y}" width="{w}" height="{h}" rx="3" ry="3" stroke-dasharray="3 2"/></g>"#,
                x = fmt_num(x),
                y = fmt_num(swatch_top),
                w = fmt_num(LEGEND_SWATCH_WIDTH),
                h = fmt_num(LEGEND_SWATCH_HEIGHT),
            );
        }
        LegendRow::Birth => {
            let _ = write!(
                out,
                r#"<path class="kul-edge" data-link-kind="birth" fill="none" d="M {x1} {y} L {x2} {y}"/>"#,
                x1 = fmt_num(x),
                x2 = fmt_num(x + LEGEND_SWATCH_WIDTH),
                y = fmt_num(center_y),
            );
        }
        LegendRow::Adoption => {
            // Adoption inline dasharray mirrors the production edge.
            let _ = write!(
                out,
                r#"<path class="kul-edge" data-link-kind="adoption" fill="none" d="M {x1} {y} L {x2} {y}" stroke-dasharray="6 4"/>"#,
                x1 = fmt_num(x),
                x2 = fmt_num(x + LEGEND_SWATCH_WIDTH),
                y = fmt_num(center_y),
            );
        }
        LegendRow::Marriage => {
            let _ = write!(
                out,
                r#"<path class="kul-edge" data-link-kind="marriage" fill="none" d="M {x1} {y} L {x2} {y}"/>"#,
                x1 = fmt_num(x),
                x2 = fmt_num(x + LEGEND_SWATCH_WIDTH),
                y = fmt_num(center_y),
            );
        }
        LegendRow::EndedMarriage => {
            let _ = write!(
                out,
                r#"<path class="kul-edge" data-link-kind="marriage" data-is-ended="true" fill="none" d="M {x1} {y} L {x2} {y}"/>"#,
                x1 = fmt_num(x),
                x2 = fmt_num(x + LEGEND_SWATCH_WIDTH),
                y = fmt_num(center_y),
            );
        }
    }
}

/// Convert an orthogonal polyline (each segment axis-aligned) into an
/// SVG path string with each interior corner rounded by a
/// quadratic-Bézier arc of approximately `radius` pixels.
///
/// Two-point polylines pass through as a straight line; a polyline
/// whose adjacent segments are shorter than `2 * radius` shrinks the
/// arc to fit. Returned string is the contents of the `d=` attribute.
fn polyline_to_rounded_path(points: &[(f64, f64)], radius: f64) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut path = String::with_capacity(points.len() * 16);
    let _ = write!(path, "M {} {}", fmt_num(points[0].0), fmt_num(points[0].1));
    if points.len() == 1 {
        return path;
    }
    if points.len() == 2 {
        let _ = write!(path, " L {} {}", fmt_num(points[1].0), fmt_num(points[1].1));
        return path;
    }
    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let here = points[i];
        let next = points[i + 1];
        let dx_in = here.0 - prev.0;
        let dy_in = here.1 - prev.1;
        let len_in = (dx_in * dx_in + dy_in * dy_in).sqrt();
        let dx_out = next.0 - here.0;
        let dy_out = next.1 - here.1;
        let len_out = (dx_out * dx_out + dy_out * dy_out).sqrt();
        if len_in == 0.0 || len_out == 0.0 {
            // Degenerate: emit a hard corner.
            let _ = write!(path, " L {} {}", fmt_num(here.0), fmt_num(here.1));
            continue;
        }
        // Effective radius can't exceed half of either adjacent
        // segment, so the arcs never overlap each other or shoot
        // past the segment endpoints.
        let r = radius.min(len_in / 2.0).min(len_out / 2.0);
        let arrive_x = here.0 - dx_in / len_in * r;
        let arrive_y = here.1 - dy_in / len_in * r;
        let depart_x = here.0 + dx_out / len_out * r;
        let depart_y = here.1 + dy_out / len_out * r;
        let _ = write!(
            path,
            " L {} {} Q {} {} {} {}",
            fmt_num(arrive_x),
            fmt_num(arrive_y),
            fmt_num(here.0),
            fmt_num(here.1),
            fmt_num(depart_x),
            fmt_num(depart_y),
        );
    }
    let last = points[points.len() - 1];
    let _ = write!(path, " L {} {}", fmt_num(last.0), fmt_num(last.1));
    path
}

/// Format a float without trailing zeros or trailing decimal points.
/// Layout produces integer-multiples of pixel constants in v1; this
/// keeps the snapshot tidy without forcing every coordinate through a
/// full ryu round-trip.
fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{:.0}", n)
    } else {
        // Round to 3 decimals so snapshots stay stable under
        // f64-level rounding drift; trims trailing zeros below.
        let raw = format!("{:.3}", n);
        let trimmed = raw.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_owned()
    }
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}
