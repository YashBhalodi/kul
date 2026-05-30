//! Theme-agnostic SVG emitter for the canonical UI pattern.
//!
//! Projects [`kul_layout::PositionedShape`] into an SVG string. Output
//! is theme-agnostic (ADR-0016): no inline colours; every element
//! carries a semantic CSS class plus `data-*` attributes for each
//! property (ADR-0021). Structural dasharrays (birth/adoption,
//! canonical/ghost) ship inline because they encode *what the element
//! is*, not its theme.
//!
//! # Class + attribute vocabulary
//!
//! Stable seam for consuming surfaces:
//!
//! - `kul-card` — `data-person-id`, `data-kind="canonical|ghost"`,
//!   `data-ghost-reason` (ghost only), `data-gender`, `data-is-alive`,
//!   `data-born`, `data-died`, `data-family`, `data-given`,
//!   `data-generation`.
//! - `kul-edge` — `data-link-kind="birth|adoption|marriage"`,
//!   `data-marriage-id`; for birth/adoption `data-child-id`,
//!   `data-is-past`, plus adoption's `data-adoption-start` /
//!   `data-adoption-end`; for marriage (ADR-0020) `data-host-id`,
//!   `data-joining-id`, `data-start`, `data-end`, `data-end-reason`,
//!   `data-is-ended`.
//! - `kul-label-name` — the card's name `<text>`.

mod emit;

pub use emit::ThemeConfig;

use kul_layout::PositionedShape;

/// Project a positioned shape into a theme-agnostic SVG string.
///
/// Returns a complete `<svg ...>…</svg>` element with no inline colours
/// and no script (ADR-0016). Opt in to baked styling via
/// [`ThemeConfig::self_contained`].
pub fn render(positioned: &PositionedShape, config: &ThemeConfig) -> String {
    emit::render(positioned, config)
}
