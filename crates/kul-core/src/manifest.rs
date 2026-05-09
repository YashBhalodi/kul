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
