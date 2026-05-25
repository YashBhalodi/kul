//! Theme-agnostic SVG emitter for the canonical UI pattern.
//!
//! `kul-layout` produces a [`kul_layout::PositionedShape`] (cards and
//! edge polylines in absolute pixel coordinates). This crate is the
//! final step: project that shape into an SVG string a surface
//! consumer can drop into a webview, an HTML page, an `<img>` tag, or a
//! self-contained file.
//!
//! The emitted SVG is **theme-agnostic** ([ADR-0019](../../docs/adr/0019-kul-svg-crate-boundary.md),
//! [ADR-0020](../../docs/adr/0020-canonical-visual-vs-interaction-chrome.md)):
//!
//! - No inline `fill=` / `stroke=` / `color=`. Every visual element
//!   carries a semantic CSS class name; theming is applied by the
//!   consuming surface via a stylesheet.
//! - Structural visual axes (P5 birth-vs-adoption dasharray, P15
//!   ghost-card dasharray + ↺ badge) ship in the SVG directly because
//!   they encode *what the element is*, not its theme.
//! - Edge routing is orthogonal right-angle for `InTree` edges,
//!   matching the classical descendency-tree convention (P1). Cross-
//!   tree edges (`PositionedEdge::routing == CrossTree`) land in F5.
//!
//! # Class vocabulary
//!
//! The stable seam consuming surfaces hook into:
//!
//! - `kul-card`, `kul-card--canonical`, `kul-card--ghost`
//! - `kul-edge`, `kul-edge--birth`, `kul-edge--adoption`,
//!   `kul-edge--marriage` (the thick unified marriage connector —
//!   monogamy horizontal segment or polygamy hub→spouse fan edge,
//!   ADR-0027), `kul-edge--ended` (an ended monogamy marriage edge,
//!   rendered translucent)
//! - `kul-label-name`, `kul-ghost-badge`

mod emit;

pub use emit::ThemeConfig;

use kul_layout::PositionedShape;

/// Project a positioned shape into a theme-agnostic SVG string.
///
/// The returned string is a complete `<svg ...>…</svg>` element with no
/// inline colours and no script. Drop it into an HTML body, render it
/// into an `<img>` src via a data URL, or wrap it in a default
/// stylesheet for a self-contained file (per [ADR-0019](../../docs/adr/0019-kul-svg-crate-boundary.md)).
pub fn render(positioned: &PositionedShape, config: &ThemeConfig) -> String {
    emit::render(positioned, config)
}
