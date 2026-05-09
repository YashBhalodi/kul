//! Manifest loading for the CLI.
//!
//! Each subcommand resolves the [`kul_core::manifest::Manifest`] for a
//! given input path before calling `check`. Discovery is directory-scoped
//! per [`spec/14-project-manifest.md`](../../../../spec/14-project-manifest.md):
//! the manifest for `<dir>/<file>.kul` is `<dir>/kul.yml`. When the CLI
//! reads from stdin, the caller must pass `--manifest <path>` because there
//! is no on-disk path to derive the location from.

use std::fmt;
use std::path::{Path, PathBuf};

use kul_core::manifest::Manifest;

#[derive(Debug)]
pub enum ManifestError {
    /// Stdin input given without `--manifest <path>`.
    StdinNeedsExplicitManifest,
    /// `kul.yml` missing at the resolved location.
    Missing { path: PathBuf },
    /// `kul.yml` could not be read off disk.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// `kul.yml` parsed but the YAML structure was malformed.
    Parse { path: PathBuf, message: String },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::StdinNeedsExplicitManifest => write!(
                f,
                "reading from stdin requires --manifest <path> (no on-disk file to discover kul.yml from)"
            ),
            ManifestError::Missing { path } => write!(
                f,
                "missing project manifest: expected {} alongside the input (a .kul file requires a sibling kul.yml)",
                path.display()
            ),
            ManifestError::Io { path, source } => {
                write!(f, "read {}: {source}", path.display())
            }
            ManifestError::Parse { path, message } => {
                write!(f, "parse {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for ManifestError {}

/// Resolve and load the manifest for an input.
///
/// - When `explicit` is `Some(path)`, that path is used verbatim.
/// - When `explicit` is `None` and the input is `-` (stdin), an error is
///   returned demanding `--manifest`.
/// - Otherwise the manifest path is `<input parent>/kul.yml`.
pub fn load_for(input: &Path, explicit: Option<&Path>) -> Result<Manifest, ManifestError> {
    let manifest_path = resolve_path(input, explicit)?;
    if !manifest_path.exists() {
        return Err(ManifestError::Missing {
            path: manifest_path,
        });
    }
    let raw = std::fs::read_to_string(&manifest_path).map_err(|err| ManifestError::Io {
        path: manifest_path.clone(),
        source: err,
    })?;
    parse(&raw).map_err(|message| ManifestError::Parse {
        path: manifest_path,
        message,
    })
}

fn resolve_path(input: &Path, explicit: Option<&Path>) -> Result<PathBuf, ManifestError> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }
    if input == Path::new("-") {
        return Err(ManifestError::StdinNeedsExplicitManifest);
    }
    let parent = input.parent().unwrap_or_else(|| Path::new(""));
    Ok(parent.join("kul.yml"))
}

fn parse(raw: &str) -> Result<Manifest, String> {
    serde_yaml::from_str(raw).map_err(|err| format!("invalid YAML: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let m = parse("kul: \"0.1\"\n").unwrap();
        assert_eq!(m.kul_version, "0.1");
    }

    #[test]
    fn parse_unknown_fields_are_tolerated() {
        let m = parse("kul: \"0.1\"\nunknown: ignored\n").unwrap();
        assert_eq!(m.kul_version, "0.1");
    }

    #[test]
    fn parse_comments_are_dropped() {
        let m = parse("# leading comment\nkul: \"0.1\"  # trailing\n").unwrap();
        assert_eq!(m.kul_version, "0.1");
    }

    #[test]
    fn parse_missing_kul_field_errors() {
        let err = parse("foo: bar\n").unwrap_err();
        assert!(err.contains("invalid YAML"), "unexpected error: {err}");
    }
}
