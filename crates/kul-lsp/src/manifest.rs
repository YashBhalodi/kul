//! Manifest loading for the LSP, keyed by the `.kul` URI.
//!
//! Per [`spec/14-project-manifest.md`](../../../spec/14-project-manifest.md),
//! the manifest for a `.kul` URI is `kul.yml` in the same directory. The
//! LSP loads it once at `did_open` time and caches it alongside the parsed
//! document. Editing `kul.yml` while a `.kul` URI is open requires
//! close/reopen to take effect — issue 63's multi-file work will revisit.

use std::fmt;
use std::path::PathBuf;

use kul_core::manifest::Manifest;
use tower_lsp::lsp_types::Url;

/// Failure modes the LSP surfaces as a single synthetic diagnostic at byte
/// `0..1` of the `.kul` URI.
#[derive(Debug)]
pub enum ManifestError {
    /// `.kul` URI is not a `file://` URI; we have no on-disk path to
    /// derive `kul.yml`'s location from.
    NotAFileUrl,
    /// `kul.yml` missing at the resolved location.
    Missing { path: PathBuf },
    /// `kul.yml` could not be read off disk.
    Io { path: PathBuf, message: String },
    /// `kul.yml` parsed but the YAML structure was malformed.
    Parse { path: PathBuf, message: String },
}

impl ManifestError {
    /// Diagnostic-message text. The LSP wraps this in a `Diagnostic` at
    /// byte 0..1 of the URI; semantic and validation are skipped.
    pub fn message(&self) -> String {
        match self {
            ManifestError::NotAFileUrl => {
                "this Kul URI is not a `file://` URL; manifest discovery requires an on-disk path"
                    .to_string()
            }
            ManifestError::Missing { path } => format!(
                "missing project manifest: expected {} alongside this file (a .kul file requires a sibling kul.yml)",
                path.display()
            ),
            ManifestError::Io { path, message } => {
                format!("read {}: {message}", path.display())
            }
            ManifestError::Parse { path, message } => {
                format!("parse {}: {message}", path.display())
            }
        }
    }
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for ManifestError {}

/// Resolve and load the manifest for a `.kul` URI.
///
/// The lookup rule is `<.kul URI parent>/kul.yml`. Non-`file://` URIs are
/// rejected; programmatic clients must speak in terms of an on-disk path.
pub fn load_for(uri: &Url) -> Result<Manifest, ManifestError> {
    let kul_path = uri.to_file_path().map_err(|_| ManifestError::NotAFileUrl)?;
    let parent = kul_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""));
    let manifest_path = parent.join("kul.yml");
    if !manifest_path.exists() {
        return Err(ManifestError::Missing {
            path: manifest_path,
        });
    }
    let raw = std::fs::read_to_string(&manifest_path).map_err(|err| ManifestError::Io {
        path: manifest_path.clone(),
        message: err.to_string(),
    })?;
    serde_yaml::from_str(&raw).map_err(|err| ManifestError::Parse {
        path: manifest_path.clone(),
        message: format!("invalid YAML: {err}"),
    })
}
