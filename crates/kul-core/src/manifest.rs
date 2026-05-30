//! Project manifest (`kul.yml`).
//!
//! A Kul project is `kul.yml` plus one or more sibling `.kul` files. The
//! manifest carries the language version, lifted out of the grammar so
//! the DSL contains only kinship. See
//! [`spec/14-project-manifest.md`](../../../spec/14-project-manifest.md)
//! and ADR-0013.
//!
//! Adapters own filesystem I/O; `kul-core` never reads the FS. [`parse`]
//! is the thin YAML→typed helper; [`validate`] is the richer pass
//! [`crate::check`] uses, producing a [`Manifest`] plus `KUL-M02..M05`
//! diagnostics. `KUL-M01` (manifest missing) is unanchored and owned by
//! the adapter.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
#[cfg(feature = "tsify")]
use tsify::Tsify;

#[cfg(feature = "yaml")]
use crate::diagnostic::{Diagnostic, fspan, manifest_codes};
use crate::export::LANGUAGE_VERSION;
#[cfg(feature = "yaml")]
use crate::span::{ByteSpan, FileId};

/// Resolve the manifest path for a `.kul` input: `<dir>/kul.yml`, same
/// directory, no walk-up (spec §14.3). Pure path manipulation, no I/O.
#[must_use]
pub fn sibling_path(input: &Path) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new(""));
    parent.join("kul.yml")
}

/// Language versions accepted in the manifest's `kul:` field.
#[cfg(feature = "yaml")]
const RECOGNIZED_VERSIONS: &[&str] = &[LANGUAGE_VERSION];

/// Top-level manifest fields the v1 schema knows; anything else fires
/// `KUL-M05` (warning).
#[cfg(feature = "yaml")]
const KNOWN_FIELDS: &[&str] = &["kul"];

/// Typed `kul.yml` manifest. Serialized with the `kul:` field name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(from_wasm_abi, into_wasm_abi))]
pub struct Manifest {
    /// Language version (`MAJOR.MINOR`) the sibling `.kul` files target.
    /// Surfaced in the export envelope's `kul:` field.
    #[serde(rename = "kul")]
    pub kul_version: String,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            kul_version: LANGUAGE_VERSION.to_string(),
        }
    }
}

/// YAML parse failure carrying the message and (when available) the
/// 0-indexed line/column for `KUL-M02` anchoring.
#[cfg(feature = "yaml")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
    location: Option<(usize, usize)>,
}

#[cfg(feature = "yaml")]
impl ParseError {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 0-indexed `(line, column)` from the YAML parser.
    #[must_use]
    pub fn location(&self) -> Option<(usize, usize)> {
        self.location
    }
}

#[cfg(feature = "yaml")]
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

#[cfg(feature = "yaml")]
impl std::error::Error for ParseError {}

/// Parse a `kul.yml` payload into a typed [`Manifest`]. Unknown fields
/// are tolerated (ADR-0013). For the diagnostic-producing pass, see
/// [`validate`].
#[cfg(feature = "yaml")]
#[must_use = "parsing the manifest is pointless if the result is discarded"]
pub fn parse(yaml: &str) -> Result<Manifest, ParseError> {
    serde_yaml::from_str(yaml).map_err(|err| ParseError {
        message: format!("invalid YAML: {err}"),
        location: err.location().map(|loc| (loc.line(), loc.column())),
    })
}

/// Project-level manifest validator. Returns a typed [`Manifest`] (when
/// assemblable) plus `KUL-M02..M05` diagnostics anchored at
/// `manifest_file`. `KUL-M01` is owned by the adapter (kul-core doesn't
/// see the filesystem).
#[cfg(feature = "yaml")]
#[must_use]
pub fn validate(yaml: &str, manifest_file: FileId) -> (Option<Manifest>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    if yaml.is_empty() {
        // In-memory caller without a real `kul.yml`; fall back to
        // [`Manifest::default`] silently.
        return (None, diagnostics);
    }
    let value: serde_yaml::Value = match serde_yaml::from_str(yaml) {
        Ok(v) => v,
        Err(err) => {
            let span = serde_yaml_span(yaml, err.location());
            diagnostics.push(Diagnostic::error(
                manifest_codes::M02_MALFORMED_YAML,
                format!("invalid manifest YAML: {err}"),
                fspan(manifest_file, span),
            ));
            return (None, diagnostics);
        }
    };

    let mapping = match &value {
        serde_yaml::Value::Mapping(m) => m,
        _ => {
            // YAML parsed but not a mapping (e.g. bare scalar). Surface
            // as M02 — structurally valid YAML but not a manifest.
            let span = ByteSpan::new(0, yaml.len().min(1));
            diagnostics.push(Diagnostic::error(
                manifest_codes::M02_MALFORMED_YAML,
                "manifest must be a YAML mapping (a top-level `kul:` field)".to_string(),
                fspan(manifest_file, span),
            ));
            return (None, diagnostics);
        }
    };

    let mut kul_value: Option<&serde_yaml::Value> = None;
    for (k, v) in mapping {
        let key = match k.as_str() {
            Some(s) => s,
            None => continue,
        };
        if key == "kul" {
            kul_value = Some(v);
        } else if !KNOWN_FIELDS.contains(&key) {
            let span = locate_key(yaml, key);
            diagnostics.push(Diagnostic::warning(
                manifest_codes::M05_UNKNOWN_FIELD,
                format!("unknown manifest field `{key}` (ignored; v1 schema has only `kul:`)"),
                fspan(manifest_file, span),
            ));
        }
    }

    let kul_version = match kul_value {
        None => {
            diagnostics.push(Diagnostic::error(
                manifest_codes::M03_MISSING_KUL_FIELD,
                "manifest is missing required `kul:` field — the language version this project targets",
                fspan(manifest_file, ByteSpan::new(0, yaml.len().min(1))),
            ));
            return (None, diagnostics);
        }
        Some(v) => match v {
            serde_yaml::Value::String(s) => s.clone(),
            other => {
                let span = locate_key_value(yaml, "kul");
                let raw = serde_yaml::to_string(other)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                diagnostics.push(Diagnostic::error(
                    manifest_codes::M04_UNKNOWN_VERSION,
                    format!(
                        "manifest `kul:` value `{raw}` is not a recognized Kul language version (expected one of: {})",
                        RECOGNIZED_VERSIONS.join(", ")
                    ),
                    fspan(manifest_file, span),
                ));
                return (None, diagnostics);
            }
        },
    };

    if !RECOGNIZED_VERSIONS.iter().any(|v| *v == kul_version) {
        let span = locate_key_value(yaml, "kul");
        diagnostics.push(Diagnostic::error(
            manifest_codes::M04_UNKNOWN_VERSION,
            format!(
                "manifest `kul:` value `{kul_version}` is not a recognized Kul language version (expected one of: {})",
                RECOGNIZED_VERSIONS.join(", ")
            ),
            fspan(manifest_file, span),
        ));
        return (None, diagnostics);
    }

    (Some(Manifest { kul_version }), diagnostics)
}

/// `serde_yaml` location → one-byte [`ByteSpan`] for KUL-M02 anchoring.
#[cfg(feature = "yaml")]
fn serde_yaml_span(yaml: &str, loc: Option<serde_yaml::Location>) -> ByteSpan {
    let Some(loc) = loc else {
        return ByteSpan::new(0, yaml.len().min(1));
    };
    let start = byte_offset_at(yaml, loc.line(), loc.column());
    ByteSpan::new(start, (start + 1).min(yaml.len()))
}

/// 0-indexed `(line, column)` → byte offset; clamps to end of source.
#[cfg(feature = "yaml")]
fn byte_offset_at(yaml: &str, line: usize, column: usize) -> usize {
    let mut current_line = 0usize;
    for (i, b) in yaml.bytes().enumerate() {
        if current_line == line {
            return (i + column).min(yaml.len());
        }
        if b == b'\n' {
            current_line += 1;
        }
    }
    yaml.len()
}

/// Byte span of a top-level `<key>:` token. Best-effort; returns `0..0`
/// if not at column 0.
#[cfg(feature = "yaml")]
fn locate_key(yaml: &str, key: &str) -> ByteSpan {
    let needle_a = format!("\n{key}:");
    let needle_b = format!("{key}:");
    if yaml.starts_with(&needle_b) {
        return ByteSpan::new(0, key.len());
    }
    if let Some(pos) = yaml.find(&needle_a) {
        let start = pos + 1;
        return ByteSpan::new(start, start + key.len());
    }
    ByteSpan::new(0, 0)
}

/// Byte span of a top-level `<key>:` value (the text after `:`).
#[cfg(feature = "yaml")]
fn locate_key_value(yaml: &str, key: &str) -> ByteSpan {
    let key_span = locate_key(yaml, key);
    if key_span.is_empty() {
        return key_span;
    }
    let after_key = (key_span.end + 1).min(yaml.len());
    let bytes = yaml.as_bytes();
    let mut start = after_key;
    while start < bytes.len() && (bytes[start] == b' ' || bytes[start] == b'\t') {
        start += 1;
    }
    let mut end = start;
    while end < bytes.len() && bytes[end] != b'\n' && bytes[end] != b'\r' {
        end += 1;
    }
    while end > start && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
        end -= 1;
    }
    if end == start {
        end = (start + 1).min(yaml.len());
    }
    ByteSpan::new(start, end)
}

#[cfg(all(test, feature = "yaml"))]
mod tests {
    use super::*;

    #[test]
    fn sibling_path_is_kul_yml_in_same_directory() {
        assert_eq!(
            sibling_path(Path::new("examples/04/family.kul")),
            PathBuf::from("examples/04/kul.yml")
        );
    }

    #[test]
    fn sibling_path_handles_bare_filename_without_parent() {
        assert_eq!(
            sibling_path(Path::new("family.kul")),
            PathBuf::from("kul.yml")
        );
    }

    #[test]
    fn sibling_path_handles_absolute_paths() {
        let got = sibling_path(Path::new("/srv/project/family.kul"));
        assert_eq!(got, PathBuf::from("/srv/project/kul.yml"));
    }

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
    fn parse_missing_kul_field_errors() {
        let err = parse("foo: bar\n").unwrap_err();
        assert!(
            err.message().contains("invalid YAML"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validate_minimal_manifest_clean() {
        let (m, diags) = validate("kul: \"0.1\"\n", FileId::MANIFEST);
        assert_eq!(m.expect("manifest").kul_version, "0.1");
        assert!(diags.is_empty(), "expected no diagnostics, got {diags:#?}");
    }

    #[test]
    fn validate_unknown_field_emits_m05_warning() {
        let (m, diags) = validate("kul: \"0.1\"\nfoo: bar\n", FileId::MANIFEST);
        assert!(m.is_some());
        let m05: Vec<_> = diags.iter().filter(|d| d.code == "KUL-M05").collect();
        assert_eq!(m05.len(), 1);
        assert_eq!(
            m05[0].severity,
            crate::diagnostic::Severity::Warning,
            "M05 must be a warning"
        );
    }

    #[test]
    fn validate_missing_kul_field_emits_m03() {
        let (m, diags) = validate("foo: bar\n", FileId::MANIFEST);
        assert!(m.is_none());
        assert!(diags.iter().any(|d| d.code == "KUL-M03"));
    }

    #[test]
    fn validate_unknown_version_emits_m04() {
        let (m, diags) = validate("kul: \"99.9\"\n", FileId::MANIFEST);
        assert!(m.is_none());
        assert!(diags.iter().any(|d| d.code == "KUL-M04"));
    }

    #[test]
    fn validate_malformed_yaml_emits_m02() {
        let (m, diags) = validate("kul: [unterminated\n", FileId::MANIFEST);
        assert!(m.is_none());
        assert!(diags.iter().any(|d| d.code == "KUL-M02"));
    }

    #[test]
    fn validate_empty_yaml_returns_no_diagnostics_and_no_manifest() {
        let (m, diags) = validate("", FileId::MANIFEST);
        assert!(m.is_none());
        assert!(diags.is_empty());
    }
}
