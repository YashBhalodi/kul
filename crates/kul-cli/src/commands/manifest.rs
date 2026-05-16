//! Manifest discovery for the CLI.
//!
//! Each subcommand resolves the manifest for a given input path before
//! calling `check`. Discovery is directory-scoped per
//! [`spec/14-project-manifest.md`](../../../../spec/14-project-manifest.md):
//! the manifest for `<dir>/<file>.kul` is `<dir>/kul.yml`. Per the
//! file-identity refactor (issue #70), the CLI hands `kul_core::check`
//! the **raw YAML bytes** alongside the manifest path label so manifest
//! diagnostics flow through the standard `RenderableDiagnostic` rendering
//! path with `KUL-Mxx` codes — the previous string-based error rendering
//! is gone.

use std::path::Path;

use kul_core::diagnostic::{Diagnostic, manifest_codes};
use kul_core::manifest::sibling_path;

/// The discovered manifest for an input.
///
/// `path_label` is the canonical name `kul-core` uses for the manifest
/// `FileId` (it shows up in JSON `file:` fields, miette source-block
/// headings, etc.). `yaml` is the raw bytes (empty when the file was not
/// readable). `m01` is a synthetic `KUL-M01` diagnostic the CLI prepends
/// to the diagnostic stream when the manifest was missing on disk —
/// that diagnostic has no anchor, so the CLI surfaces it through the
/// standard renderer with no source-code block.
pub struct ManifestPayload {
    pub path_label: String,
    pub yaml: String,
    pub preface: Vec<Diagnostic>,
}

/// Resolve the manifest for `input` and read its bytes off disk. The
/// returned [`ManifestPayload`] always carries a stable `path_label` so
/// manifest-anchored diagnostics ("missing required field `kul:`")
/// render with the right filename header.
pub fn load_for(input: &Path) -> ManifestPayload {
    let manifest_path = sibling_path(input);
    let path_label = manifest_path.to_string_lossy().into_owned();
    if !manifest_path.exists() {
        let preface = vec![Diagnostic::unanchored_error(
            manifest_codes::M01_MISSING,
            format!(
                "missing project manifest: expected {path_label} alongside the input \
                 (a .kul file requires a sibling kul.yml)"
            ),
        )];
        return ManifestPayload {
            path_label,
            yaml: String::new(),
            preface,
        };
    }
    match std::fs::read_to_string(&manifest_path) {
        Ok(yaml) => ManifestPayload {
            path_label,
            yaml,
            preface: Vec::new(),
        },
        Err(err) => {
            // Surface IO failures as M01 too; the underlying details
            // belong in the message (the `KUL-M01` code is
            // "manifest unavailable" in practice).
            let preface = vec![Diagnostic::unanchored_error(
                manifest_codes::M01_MISSING,
                format!("failed to read project manifest {path_label}: {err}"),
            )];
            ManifestPayload {
                path_label,
                yaml: String::new(),
                preface,
            }
        }
    }
}
