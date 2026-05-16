//! Manifest loading for the LSP, keyed by the `.kul` URI.
//!
//! Per [`spec/14-project-manifest.md`](../../../spec/14-project-manifest.md),
//! the manifest for a `.kul` URI is `kul.yml` in the same directory. The
//! LSP loads it before each `kul_core::check` call and feeds the raw YAML
//! bytes plus the resolved label into the multi-file check pipeline; the
//! pipeline then routes `KUL-Mxx` diagnostics through the standard
//! diagnostic channel (anchored at `FileId::MANIFEST`).
//!
//! Editing `kul.yml` while a `.kul` URI is open requires close/reopen to
//! take effect from the editor's POV (file-watching is issue 63's
//! territory); our `did_change` handler still re-loads the manifest so an
//! external write before the next keystroke takes effect on the next
//! check.

use kul_core::manifest::sibling_path;
use tower_lsp::lsp_types::Url;

/// Resolve and load the manifest for a `.kul` URI. Returns the manifest
/// `(label, yaml-bytes)` pair the multi-file check expects.
///
/// Failure modes (URI scheme other than `file://`, missing `kul.yml`,
/// IO errors) collapse to "no manifest body" — the LSP feeds an empty
/// YAML string into `kul_core::check`, which returns `Manifest::default`
/// without emitting a diagnostic. The LSP layers its own
/// "missing/unreadable manifest" notice on top: surfacing the fact that
/// the editor couldn't reach the file is the LSP's job, not the
/// language core's. (Once issue 63's file-watching lands the surface for
/// missing-on-disk will tighten.)
pub fn manifest_yaml_for(uri: &Url) -> (String, String) {
    let kul_path = match uri.to_file_path() {
        Ok(p) => p,
        Err(_) => return (uri.to_string(), String::new()),
    };
    let manifest_path = sibling_path(&kul_path);
    let label = manifest_path.to_string_lossy().into_owned();
    match std::fs::read_to_string(&manifest_path) {
        Ok(yaml) => (label, yaml),
        Err(_) => (label, String::new()),
    }
}
