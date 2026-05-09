//! Project manifest (`kul.yml`).
//!
//! A Kul project is "a `kul.yml` plus one or more `.kul` files in the same
//! directory." The manifest carries metadata *about* the source — most
//! notably the Kul language version the sibling `.kul` files target —
//! lifted out of the grammar so the DSL itself contains only kinship.
//!
//! The manifest is normative (every Kul-language consumer honours it) and
//! required (a `.kul` file without a sibling `kul.yml` is not a valid Kul
//! project). See [`spec/14-project-manifest.md`](../../../spec/14-project-manifest.md)
//! and [ADR-0013](../../../docs/adr/0013-project-manifest.md) for the
//! load-bearing decisions.
//!
//! Adapters own filesystem / JS-host I/O. `kul-core` itself never reads
//! the filesystem; the [`parse`] helper turns a YAML string the adapter
//! has already loaded into a typed [`Manifest`].

use serde::{Deserialize, Serialize};
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::export::LANGUAGE_VERSION;

/// Typed representation of a `kul.yml` manifest.
///
/// One field today (`kul_version`); the manifest schema evolves alongside
/// the Kul language version per the additivity principle. Adapters
/// (`kul-cli`, `kul-lsp`, `kul-wasm`) are responsible for parsing the
/// on-disk YAML / JS object into this struct; `kul-core` itself never
/// reads the filesystem.
///
/// Serializes / deserializes with the `kul:` field name (matches the
/// on-disk YAML schema and the JS object the WASM bridge accepts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(from_wasm_abi, into_wasm_abi))]
pub struct Manifest {
    /// The Kul language version that the sibling `.kul` files conform to.
    /// Format is `MAJOR.MINOR`, matching the previously-in-grammar version
    /// literal. Surfaced in the export envelope's `kul:` field.
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

/// YAML parse failure. Carries a human-readable message — adapters wrap
/// it into their own error types alongside the on-disk path or other
/// adapter-specific context.
#[cfg(feature = "yaml")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
}

#[cfg(feature = "yaml")]
impl ParseError {
    pub fn message(&self) -> &str {
        &self.message
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
/// `invalid YAML: <serde-yaml message>` body.
#[cfg(feature = "yaml")]
pub fn parse(yaml: &str) -> Result<Manifest, ParseError> {
    serde_yaml::from_str(yaml).map_err(|err| ParseError {
        message: format!("invalid YAML: {err}"),
    })
}

#[cfg(all(test, feature = "yaml"))]
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
        assert!(
            err.message().contains("invalid YAML"),
            "unexpected error: {err}",
        );
    }
}
