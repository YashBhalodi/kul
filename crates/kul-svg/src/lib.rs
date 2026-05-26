//! Theme-agnostic SVG emitter for the canonical UI pattern.
//!
//! `kul-layout` produces a [`kul_layout::PositionedShape`] (cards and
//! edge polylines in absolute pixel coordinates). This crate is the
//! final step: project that shape into an SVG string a surface
//! consumer can drop into a webview, an HTML page, an `<img>` tag, or a
//! self-contained file.
//!
//! The emitted SVG is **theme-agnostic** ([ADR-0016](../../docs/adr/0016-visualization-pipeline-crate-boundaries.md),
//! [ADR-0016](../../docs/adr/0016-visualization-pipeline-crate-boundaries.md)):
//!
//! - No inline `fill=` / `stroke=` / `color=`. Every visual element
//!   carries a semantic CSS class naming its *type*; theming is applied
//!   by the consuming surface via a stylesheet.
//! - Structural visual axes (the birth-vs-adoption dasharray of edges
//!   encode link kind, the ghost-card dasharray of the uniform card)
//!   ship in the SVG directly because they encode *what the element
//!   is*, not its theme.
//! - Every edge routes with one orthogonal right-angle geometry,
//!   matching the classical descendency-tree convention (no routing
//!   discriminator; [ADR-0018](../../docs/adr/0018-canonical-layout-algorithm.md)).
//!
//! # Class + attribute vocabulary
//!
//! Entity classes name the element *type*; every *property* is a
//! `data-*` attribute (booleans as `data-is-*`, enums as explicit
//! strings, missing optionals omitted). Every Person / Marriage /
//! birth / adoption property the language declares plumbs through to a
//! `data-*` attribute ([ADR-0021](../../docs/adr/0021-language-properties-plumb-to-svg.md)).
//! The stable seam consuming surfaces hook into:
//!
//! - `kul-card` — `data-person-id`, `data-kind="canonical|ghost"`,
//!   `data-ghost-reason` (ghost only), `data-gender`, `data-is-alive`,
//!   `data-born`, `data-died`, `data-family`, `data-given`,
//!   `data-generation`.
//! - `kul-edge` — `data-link-kind="birth|adoption|marriage"`,
//!   `data-marriage-id`; for birth / adoption `data-child-id`,
//!   `data-is-past`, and adoption's `data-adoption-start` /
//!   `data-adoption-end`; for the thick unified marriage connector
//!   (ADR-0020) `data-host-id`, `data-joining-id`, `data-start`,
//!   `data-end`, `data-end-reason`, `data-is-ended`.
//! - `kul-label-name` — the card's name `<text>`.

mod emit;

pub use emit::ThemeConfig;

use kul_layout::PositionedShape;

/// Project a positioned shape into a theme-agnostic SVG string.
///
/// The returned string is a complete `<svg ...>…</svg>` element with no
/// inline colours and no script. Drop it into an HTML body, render it
/// into an `<img>` src via a data URL, or wrap it in a default
/// stylesheet for a self-contained file (per [ADR-0016](../../docs/adr/0016-visualization-pipeline-crate-boundaries.md)).
pub fn render(positioned: &PositionedShape, config: &ThemeConfig) -> String {
    emit::render(positioned, config)
}
