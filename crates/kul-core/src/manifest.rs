//! Project manifest (`kul.yml`).
//!
//! A Kul project is "a `kul.yml` plus one or more `.kul` files in the
//! same directory." The manifest carries metadata *about* the source —
//! most notably the Kul language version the sibling `.kul` files
//! target — lifted out of the grammar so the DSL itself contains only
//! kinship.
//!
//! The manifest is normative (every Kul-language consumer honours it)
//! and required (a `.kul` file without a sibling `kul.yml` is not a
//! valid Kul project). See [`spec/14-project-manifest.md`](../../../spec/14-project-manifest.md)
//! and [ADR-0013](../../../docs/adr/0013-project-manifest.md) for the
//! load-bearing decisions.
//!
//! Adapters own filesystem / JS-host I/O. `kul-core` itself never reads
//! the filesystem; the [`parse`] helper turns a YAML string the adapter
//! has already loaded into a typed [`Manifest`]. The richer
//! [`validate`] pass is what [`crate::check`] calls — it produces a
//! [`Manifest`] *and* a diagnostic list with `KUL-M02..M05` codes
//! anchored at the manifest's [`FileId`]. `KUL-M01` (manifest missing
//! on disk) is the adapter's responsibility to detect and report; it is
//! the only manifest-related code that has no anchor in `kul.yml` (the
//! file isn't there).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
#[cfg(feature = "tsify")]
use tsify::Tsify;

#[cfg(feature = "yaml")]
use crate::diagnostic::{Diagnostic, fspan, manifest_codes};
use crate::export::LANGUAGE_VERSION;
#[cfg(feature = "yaml")]
use crate::span::{ByteSpan, FileId};

/// Resolve the project manifest path for a `.kul` input.
///
/// Per [`spec/14-project-manifest.md`](../../../spec/14-project-manifest.md)
/// §14.3, the manifest for `<dir>/<file>.kul` is `<dir>/kul.yml` — same
/// directory, no walk-up. This is the one authoritative encoding of the
/// rule; CLI and LSP adapters both call it so a future spec edit (say,
/// allowing `kul.yaml` as an alias, or supporting walk-up) lands in one
/// place.
///
/// Pure path manipulation only — no filesystem IO. ADR-0014 keeps
/// filesystem reads at the adapter layer; this function just rewrites
/// one path into another.
pub fn sibling_path(input: &Path) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new(""));
    parent.join("kul.yml")
}

/// Versions of the Kul language this `kul-core` build accepts in the
/// manifest's `kul:` field. Today only `0.1` is recognized; new versions
/// land as additive entries here in lockstep with the spec.
#[cfg(feature = "yaml")]
const RECOGNIZED_VERSIONS: &[&str] = &[LANGUAGE_VERSION];

/// Top-level field names the v1 manifest schema knows about. Any other
/// top-level key surfaces as `KUL-M05` (warning).
#[cfg(feature = "yaml")]
const KNOWN_FIELDS: &[&str] = &["kul"];

/// Typed representation of a `kul.yml` manifest.
///
/// One field today (`kul_version`); the manifest schema evolves alongside
/// the Kul language version per the additivity principle. Adapters
/// (`kul-cli`, `kul-lsp`, `kul-wasm`) are responsible for loading the
/// on-disk YAML / JS object before handing it to `kul-core`; `kul-core`
/// itself never reads the filesystem.
///
/// Serializes / deserializes with the `kul:` field name (matches the
/// on-disk YAML schema and the JS object the WASM bridge accepts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(from_wasm_abi, into_wasm_abi))]
pub struct Manifest {
    /// The Kul language version that the sibling `.kul` files conform
    /// to. Format is `MAJOR.MINOR`, matching the previously-in-grammar
    /// version literal. Surfaced in the export envelope's `kul:` field.
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

/// YAML parse failure, carrying both a human-readable message and (when
/// available) the line/column reported by `serde_yaml` so the manifest
/// validator pass can anchor a `KUL-M02` diagnostic at the right
/// position.
#[cfg(feature = "yaml")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
    /// 0-indexed line/column from `serde_yaml`. `None` when the YAML
    /// library did not surface a location (rare).
    location: Option<(usize, usize)>,
}

#[cfg(feature = "yaml")]
impl ParseError {
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 0-indexed line/column reported by the YAML parser. Returned as a
    /// pair to keep the public surface free of `serde_yaml` types.
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

/// Parse a `kul.yml` payload into a typed [`Manifest`].
///
/// Unknown fields are tolerated (per ADR-0013's additivity stance);
/// missing required fields surface as a [`ParseError`] with a
/// `invalid YAML: <serde-yaml message>` body. For the project-level
/// pass that produces normative `KUL-Mxx` diagnostics, see
/// [`validate`].
#[cfg(feature = "yaml")]
pub fn parse(yaml: &str) -> Result<Manifest, ParseError> {
    serde_yaml::from_str(yaml).map_err(|err| ParseError {
        message: format!("invalid YAML: {err}"),
        location: err.location().map(|loc| (loc.line(), loc.column())),
    })
}

/// Project-level manifest validator pass: parses `yaml` if it can, then
/// walks the document for schema-shape problems, returning a typed
/// [`Manifest`] (when one could be assembled) plus the diagnostic list
/// (`KUL-M02..M05`) anchored at `manifest_file`.
///
/// `KUL-M01` (manifest missing on disk) is *not* produced here — the
/// adapter detects that case before calling, since `kul-core` doesn't
/// see the filesystem. When the manifest body could not be parsed at
/// all, the function returns `(None, …)` with a `KUL-M02` diagnostic
/// and the caller falls back to [`Manifest::default`].
///
/// When the `yaml` feature is disabled (e.g. WASM builds that get a
/// pre-built [`Manifest`] from the JS host), this function is unavailable
/// and callers route around it through [`crate::check_with_manifest`].
#[cfg(feature = "yaml")]
pub fn validate(yaml: &str, manifest_file: FileId) -> (Option<Manifest>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    if yaml.is_empty() {
        // The manifest source is empty — the adapter is calling us as
        // part of a check pipeline that doesn't have a real `kul.yml`.
        // We fall back to the default manifest (Manifest::default uses
        // the build's LANGUAGE_VERSION) without emitting anything.
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
            // YAML parsed to something other than a mapping (a bare
            // scalar at the document root, for instance). Surface it as
            // M02 — the YAML is structurally valid but not a manifest.
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

/// Convert a `serde_yaml` (line, column) into a [`ByteSpan`] covering
/// one byte at that position. Used as the anchor for `KUL-M02`.
#[cfg(feature = "yaml")]
fn serde_yaml_span(yaml: &str, loc: Option<serde_yaml::Location>) -> ByteSpan {
    let Some(loc) = loc else {
        return ByteSpan::new(0, yaml.len().min(1));
    };
    let start = byte_offset_at(yaml, loc.line(), loc.column());
    ByteSpan::new(start, (start + 1).min(yaml.len()))
}

/// Map a 0-indexed `(line, column)` into a byte offset in `yaml`. Out-of-
/// range positions clamp to the end of the source.
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

/// Find the byte span of a top-level `<key>:` token in `yaml`. Best-
/// effort: if the literal `<key>:` is not at column 0 of any line,
/// returns `0..0` (rare; fires only on hand-crafted malformed input).
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

/// Find the byte span of a top-level `<key>:` *value* in `yaml`. Like
/// [`locate_key`] but anchors on the value text after the `:`.
#[cfg(feature = "yaml")]
fn locate_key_value(yaml: &str, key: &str) -> ByteSpan {
    let key_span = locate_key(yaml, key);
    if key_span.is_empty() {
        return key_span;
    }
    let after_key = key_span.end + 1; // skip the `:`
    let after_key = after_key.min(yaml.len());
    // Skip whitespace
    let bytes = yaml.as_bytes();
    let mut start = after_key;
    while start < bytes.len() && (bytes[start] == b' ' || bytes[start] == b'\t') {
        start += 1;
    }
    // Find end of line (or end of value: until newline or end)
    let mut end = start;
    while end < bytes.len() && bytes[end] != b'\n' && bytes[end] != b'\r' {
        end += 1;
    }
    // Trim trailing whitespace
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
        // `family.kul` has no parent — the rule is "same directory", which
        // for a relative bare filename is the current working directory.
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
        // The empty-input shortcut: in-memory callers (e.g. format
        // tooling) pass an empty `manifest_yaml` and get back the
        // default [`Manifest`] without spurious M-series diagnostics.
        let (m, diags) = validate("", FileId::MANIFEST);
        assert!(m.is_none());
        assert!(diags.is_empty());
    }
}
