//! SVG string templating from `PositionedShape`. Stateless; semantic
//! CSS classes only — no inline colours, no script.

use std::fmt::Write;

use kul_layout::{EdgeKind, PositionedCard, PositionedEdge, PositionedShape, SlotKind};
use kul_render::GhostReason;

/// Theme / emission configuration.
///
/// Forward-compatibility seam (ADR-0016): default stays theme-agnostic;
/// opt-in fields tune emission without changing [`crate::render`]'s
/// signature. The private trailing field forces construction through the
/// `with_*` builders so a new field never breaks a caller.
#[allow(clippy::manual_non_exhaustive)]
#[derive(Debug, Clone, Default)]
pub struct ThemeConfig {
    /// Bake a concrete neutral light theme into the SVG as an inline
    /// `<style>` so the file renders correctly with no external CSS.
    /// Default `false` keeps output theme-agnostic (no inline colours)
    /// per ADR-0016; only `kul export --format=svg` opts in. Excludes
    /// all surface chrome (pan/zoom, hover, selection, ghost `↺` badge).
    pub self_contained: bool,
    /// Emit a legend (canonical-pattern visual key) in a reserved band
    /// at the bottom-left. Rows appear only for categories the diagram
    /// actually surfaces (ADR-0022). Swatch colour resolves through the
    /// surrounding stylesheet, so this is typically paired with
    /// [`self_contained`](ThemeConfig::self_contained).
    pub legend: bool,
    _private: (),
}

impl ThemeConfig {
    /// Build a config with [`self_contained`](ThemeConfig::self_contained) set.
    pub fn with_self_contained(self_contained: bool) -> Self {
        Self {
            self_contained,
            ..Default::default()
        }
    }

    /// Chainable setter for [`legend`](ThemeConfig::legend).
    pub fn with_legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }
}

pub(crate) fn render(positioned: &PositionedShape, config: &ThemeConfig) -> String {
    let mut out = String::with_capacity(2048);
    let rows = if config.legend {
        legend_rows(positioned)
    } else {
        Vec::new()
    };
    let legend_extra_height = if rows.is_empty() {
        0.0
    } else {
        LEGEND_GAP
            + (rows.len() as f64) * LEGEND_ROW_HEIGHT
            + 2.0 * LEGEND_PANEL_PAD_Y
            + LEGEND_PANEL_INSET
    };
    // max() guards the degenerate case where the legend is wider than the diagram.
    let canvas_width = if rows.is_empty() {
        positioned.width
    } else {
        positioned.width.max(LEGEND_TOTAL_WIDTH)
    };
    let canvas_height = positioned.height + legend_extra_height;
    write_open(&mut out, canvas_width, canvas_height);
    // Inline stylesheet must precede every element so its `svg`-scoped tokens are in scope.
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
/// ([`ThemeConfig::self_contained`]). Structural subset of the VSCode
/// preview stylesheet; excludes all chrome (pan/zoom, hover, selection,
/// ghost `↺` badge) per ADR-0016. Structural dasharrays ship inline.
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
  --kul-legend-panel-bg: #f7f9fa;
  --kul-legend-panel-border: #cfd8dc;
  --kul-legend-panel-border-width: 1;
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
.kul-legend-bg { fill: var(--kul-legend-panel-bg); stroke: var(--kul-legend-panel-border); stroke-width: var(--kul-legend-panel-border-width); }
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
    // Class names the type only; every property is a `data-*` attribute (ADR-0016, ADR-0021).
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
    let _ = write!(
        out,
        r#" data-gender="{gender}" data-is-alive="{alive}""#,
        gender = card.gender,
        alive = card.died.is_none(),
    );
    write_opt_attr(out, "data-born", card.born.as_deref());
    write_opt_attr(out, "data-died", card.died.as_deref());
    write_opt_attr(out, "data-family", card.family.as_deref());
    write_opt_attr(out, "data-given", card.given.as_deref());
    let _ = write!(out, r#" data-generation="{}">"#, card.generation);
    // Ghost dasharray ships inline (structural, ADR-0016).
    let dash = if matches!(card.kind, SlotKind::Ghost { .. }) {
        r#" stroke-dasharray="3 2""#
    } else {
        ""
    };
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
    // Class names the type only; link kind + properties are `data-*` (ADR-0016, ADR-0021).
    let _ = write!(
        out,
        r#"<path class="kul-edge" data-marriage-id="{mid}""#,
        mid = escape_xml(&edge.marriage_id),
    );
    // Adoption dasharray ships inline (structural, ADR-0016). Marriage weight comes from CSS.
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
                r#" data-link-kind="marriage" data-host-id="{host}" data-joining-id="{joining}""#,
                host = escape_xml(host_id),
                joining = escape_xml(joining_id),
            );
            write_opt_attr(out, "data-start", start.as_deref());
            let _ = write!(out, r#" data-is-ended="{is_ended}""#);
            write_opt_attr(out, "data-end", end.as_deref());
            write_opt_attr(out, "data-end-reason", end_reason.as_deref());
        }
    }
    let d = polyline_to_rounded_path(&edge.points, EDGE_CORNER_RADIUS);
    let _ = write!(out, r#" fill="none" d="{d}"{dash}/>"#);
}

/// Write ` name="value"` (XML-escaped) when `value` is `Some`; emit nothing when `None`.
/// "Absence, not placeholders" — a missing optional omits the attribute entirely.
fn write_opt_attr(out: &mut String, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        let _ = write!(out, r#" {name}="{}""#, escape_xml(value));
    }
}

const CARD_CORNER_RADIUS: f64 = 8.0;
const EDGE_CORNER_RADIUS: f64 = 10.0;

// Legend (ADR-0022): bottom-left band, opt-in via `ThemeConfig::legend`.
// Each row reuses the production `kul-card` / `kul-edge` class + `data-*`
// attributes so the surrounding stylesheet themes swatches for free.

const LEGEND_GAP: f64 = 16.0;
const LEGEND_PANEL_INSET: f64 = 8.0;
const LEGEND_PANEL_PAD_X: f64 = 14.0;
const LEGEND_PANEL_PAD_Y: f64 = 10.0;
const LEGEND_PANEL_RADIUS: f64 = 6.0;
/// Sized so the 8.75px marriage stroke (clamped to 5px via `.kul-legend`
/// override) reads as a distinct line, not as the whole row.
const LEGEND_ROW_HEIGHT: f64 = 22.0;
const LEGEND_SWATCH_WIDTH: f64 = 30.0;
const LEGEND_SWATCH_HEIGHT: f64 = 14.0;
const LEGEND_LABEL_GAP: f64 = 10.0;
/// Unmeasured label-column budget; only matters when the diagram is
/// narrower than the legend and the viewBox max()es up.
const LEGEND_LABEL_BUDGET: f64 = 120.0;
const LEGEND_PANEL_WIDTH: f64 = LEGEND_PANEL_PAD_X
    + LEGEND_SWATCH_WIDTH
    + LEGEND_LABEL_GAP
    + LEGEND_LABEL_BUDGET
    + LEGEND_PANEL_PAD_X;
const LEGEND_TOTAL_WIDTH: f64 = LEGEND_PANEL_INSET + LEGEND_PANEL_WIDTH + LEGEND_PANEL_INSET;

/// Canonical legend categories in normative order. Each is dynamic — a
/// row appears only when the diagram contains at least one element of
/// that category.
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
    /// English label. Hardcoded here so no English ships on the
    /// always-emitted SVG (ADR-0022).
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

fn write_legend(out: &mut String, rows: &[LegendRow], panel_top: f64) {
    let panel_x = LEGEND_PANEL_INSET;
    let panel_height = 2.0 * LEGEND_PANEL_PAD_Y + (rows.len() as f64) * LEGEND_ROW_HEIGHT;
    let rows_top = panel_top + LEGEND_PANEL_PAD_Y;
    let swatch_x = panel_x + LEGEND_PANEL_PAD_X;
    let label_x = swatch_x + LEGEND_SWATCH_WIDTH + LEGEND_LABEL_GAP;
    out.push_str(r#"<g class="kul-legend">"#);
    // Panel rect drawn first so rows sit on top; fill/stroke come from CSS tokens.
    let _ = write!(
        out,
        r#"<rect class="kul-legend-bg" x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" ry="{r}"/>"#,
        x = fmt_num(panel_x),
        y = fmt_num(panel_top),
        w = fmt_num(LEGEND_PANEL_WIDTH),
        h = fmt_num(panel_height),
        r = fmt_num(LEGEND_PANEL_RADIUS),
    );
    for (i, row) in rows.iter().enumerate() {
        let row_top = rows_top + (i as f64) * LEGEND_ROW_HEIGHT;
        let center_y = row_top + LEGEND_ROW_HEIGHT / 2.0;
        let swatch_top = center_y - LEGEND_SWATCH_HEIGHT / 2.0;
        write_legend_swatch(out, *row, swatch_x, swatch_top, center_y);
        let _ = write!(
            out,
            r#"<text class="kul-legend-label" x="{x}" y="{y}" dominant-baseline="central">{label}</text>"#,
            x = fmt_num(label_x),
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
            // Carry production data-* so the CSS rule themes the stroke — no hardcoded swatch colour.
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
            // Ghost card; dashed border ships inline (structural, ADR-0016) as on a real ghost.
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

/// Convert an orthogonal polyline into an SVG path string, rounding
/// each interior corner with a quadratic-Bézier arc of approximately
/// `radius` pixels. Segments shorter than `2 * radius` shrink the arc
/// to fit. Returns the `d=` attribute contents.
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
            let _ = write!(path, " L {} {}", fmt_num(here.0), fmt_num(here.1));
            continue;
        }
        // Clamp so adjacent arcs never overlap or shoot past segment endpoints.
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
/// Rounds to 3 decimals so snapshots stay stable under f64 rounding drift.
fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{:.0}", n)
    } else {
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
