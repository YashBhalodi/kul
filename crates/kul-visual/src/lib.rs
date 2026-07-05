//! Composition facade over the visualization pipeline.
//!
//! The canonical-visual pipeline — `compute` (kul-render) → `layout`
//! (kul-layout) → `render` (kul-svg) — has one invariant success arm that
//! every SVG-producing surface reruns identically. [`render_from_check`]
//! owns that arm once; the WASM `renderSvg`, CLI `kul export --format=svg`,
//! and LSP `kul/render` + `kul/exportSvg` surfaces call it and keep only
//! their own theme choice, failure projection, and output sink.
//!
//! This crate sits **above** the four ADR-0016-pinned crates (`kul-render`,
//! `kul-layout`, `kul-svg`, and `kul-core`), composing their pinned public
//! functions without broadening any of them. The one-directional dependency
//! graph `kul-visual → {kul-svg → kul-layout → kul-render → kul-core}` is
//! preserved; the facade adds a composition layer, it does not reach into
//! any pinned surface. See ADR-0031.

use kul_core::CheckResult;
use kul_core::export::ExportedDiagnostic;
use kul_layout::{LayoutConfig, layout};
use kul_render::{RenderShape, compute};
pub use kul_svg::ThemeConfig;
use kul_svg::render;

/// Run the canonical-visual pipeline's success sequence for a checked
/// project, producing the SVG string.
///
/// Composes the pinned pipeline functions — `kul_render::compute` →
/// `kul_layout::layout` → `kul_svg::render` — with the default
/// [`LayoutConfig`] and the caller's [`ThemeConfig`]. The theme is the sole
/// pipeline parameter that varies across surfaces (theme-agnostic preview
/// vs. self-contained file export); everything upstream of `render` is
/// invariant, so it lives here.
///
/// # Errors
///
/// Returns the project's [`ExportedDiagnostic`]s when the render shape is a
/// failure (the project did not pass its checks; strict-on-errors per
/// ADR-0009). Each surface projects this list into its own failure sink —
/// a JS envelope, a miette-to-stderr render, or URI/range-anchored LSP
/// diagnostics — so the returned list is the raw material, not the final
/// shape.
pub fn render_from_check(
    check: &CheckResult,
    theme: &ThemeConfig,
) -> Result<String, Vec<ExportedDiagnostic>> {
    match compute(check) {
        RenderShape::Failure(f) => Err(f.diagnostics),
        RenderShape::Success(s) => {
            let positioned = layout(&s, &LayoutConfig::default());
            Ok(render(&positioned, theme))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kul_core::ast::InputFile;
    use kul_core::manifest::Manifest;

    fn check(source: &str) -> CheckResult {
        let files = [InputFile::new("family.kul".to_owned(), source.to_owned())];
        kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &files)
    }

    #[test]
    fn clean_project_renders_theme_agnostic_svg() {
        let check = check(
            "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\nmarriage m alice bob start:1972\n",
        );
        let svg =
            render_from_check(&check, &ThemeConfig::default()).expect("clean project renders");
        assert!(svg.starts_with("<svg"), "expected an SVG document");
        assert!(
            svg.contains("kul-card"),
            "expected the canonical card class"
        );
        assert!(
            !svg.contains("<style>"),
            "default theme is theme-agnostic (no inline style)"
        );
    }

    #[test]
    fn file_export_theme_bakes_inline_style() {
        let check = check(
            "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\nmarriage m alice bob start:1972\n",
        );
        let svg = render_from_check(&check, &ThemeConfig::for_file_export())
            .expect("clean project renders");
        assert!(
            svg.contains("<style>"),
            "file-export theme is self-contained (inline style)"
        );
    }

    #[test]
    fn failing_project_returns_diagnostics() {
        // Missing required gender on a person is an error (KUL-R03).
        let check = check("person alice gender:female\n");
        let diagnostics =
            render_from_check(&check, &ThemeConfig::default()).expect_err("failing project errors");
        assert!(
            diagnostics.iter().any(|d| d.code == "KUL-R03"),
            "expected R03 in the returned diagnostics"
        );
    }
}
