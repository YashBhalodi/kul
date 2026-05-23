//! Layout metrics — pixel dimensions for cards, bars, gaps, and rows.
//!
//! Held in [`LayoutConfig`] for forward-compatibility (future configurable
//! density or alternative-algorithm dispatch); in v1 only
//! [`LayoutConfig::default()`] is constructed by any consumer.

/// Pixel dimensions for the positioning pass.
///
/// The defaults assume a monospaced font around 14px and produce a
/// classical descendency-tree layout (P1) where cards are uniform-shape
/// (P15) and generations stack top-to-bottom.
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Width of every person card (canonical or ghost). Uniform-card
    /// constraint per P15.
    pub card_width: f64,
    /// Height of every person card.
    pub card_height: f64,
    /// Width of a marriage / adoption bar.
    pub bar_width: f64,
    /// Height of a marriage / adoption bar.
    pub bar_height: f64,
    /// Horizontal gap between a card and an adjacent marriage bar
    /// (e.g. host-card right edge to bar left edge).
    pub bar_gap: f64,
    /// Horizontal gap between adjacent sibling subtrees at any level.
    pub sibling_gap: f64,
    /// Vertical distance from one generation row's card-top to the next.
    /// Cards in generation N start at `y = N * row_height + padding`.
    pub row_height: f64,
    /// Vertical distance from a marriage bar's bottom to the
    /// horizontal bus that fans out to children. Bus sits at
    /// `bar_bottom + bus_drop`; child cards start `bus_drop` below the
    /// bus.
    pub bus_drop: f64,
    /// Outer canvas padding around the bounding box on every side.
    pub padding: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            card_width: 160.0,
            card_height: 64.0,
            bar_width: 18.0,
            bar_height: 10.0,
            bar_gap: 6.0,
            sibling_gap: 32.0,
            row_height: 160.0,
            bus_drop: 28.0,
            padding: 24.0,
        }
    }
}
