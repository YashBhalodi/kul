//! Layout metrics — pixel dimensions for cards, bars, gaps, and rows.

/// Pixel dimensions for the positioning pass.
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub card_width: f64,
    pub card_height: f64,
    pub bar_width: f64,
    pub bar_height: f64,
    /// Horizontal gap between a card and an adjacent marriage bar.
    pub bar_gap: f64,
    /// Horizontal gap between adjacent sibling subtrees.
    pub sibling_gap: f64,
    /// Vertical distance from one generation row's card-top to the next.
    pub row_height: f64,
    /// Vertical distance from a marriage bar's bottom to the
    /// horizontal bus that fans out to children.
    pub bus_drop: f64,
    /// Outer canvas padding around the bounding box.
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
