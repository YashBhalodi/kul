//! Canonical JSON export of a Kul document.
//!
//! Projects a [`CheckResult`] into an [`ExportEnvelope`] suitable for
//! downstream consumers (visualizers, web apps, scripts). The envelope is
//! strict-on-errors: if `check.has_errors()` returns true, the envelope
//! carries the diagnostic list instead of the graph. Warnings do not block.
//!
//! The graph shape is **kinship-native** — three top-level collections that
//! mirror the language's primitives:
//!
//! - `persons` — every declared person with id, name, gender, optional
//!   `family` / `given` / `born` / `died` fields.
//! - `marriages` — every declared marriage with id, the two spouse ids,
//!   `start`, optional `end` and `end_reason`.
//! - `parenthood_links` — every `birth` and `adoption` sub-statement, each
//!   carrying the marriage id, the child id, and a `kind` discriminator.
//!
//! Cross-references are by id only — there are no embedded objects or
//! derived projections (e.g. `person.children`). Consumers compose those
//! views over the flat collections.
//!
//! Dates are tagged: `{ value, precision, circa }` rather than a flat ISO
//! string. The value is `YYYY[-MM[-DD]]` without the `~` prefix; the
//! `circa` flag carries the modifier separately so consumers can render
//! `~1980` as `c. 1980` (or whatever they prefer) without parsing the
//! string.
//!
//! See [`spec/16-export-schema.md`](../../../spec/16-export-schema.md) for
//! the normative schema and [ADR-0008](../../../docs/adr/0008-export-kinship-native-shape.md),
//! [ADR-0009](../../../docs/adr/0009-export-strict-on-diagnostics.md), and
//! [ADR-0010](../../../docs/adr/0010-export-schema-versioning.md) for the
//! load-bearing decisions.

pub mod cytoscape;

use serde::{Deserialize, Serialize};
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::CheckResult;
use crate::ast::{Document, EndReason, Gender, PersonStmt};
use crate::date::DateLit;
use crate::diagnostic::{Diagnostic, Severity};
use crate::semantic::ResolvedDocument;
use crate::span::{ByteSpan, FileId, FileSpan, SourceMap};
use std::collections::HashMap;

/// The export schema version. Bumped only when consumers might silently
/// mis-represent data by ignoring a new construct (e.g. a brand-new
/// top-level collection). Adding optional fields, new enum values, or new
/// `parenthood_links.kind` values does NOT bump the schema — consumers
/// handle these as forward-compatible additions per
/// [ADR-0010](../../../docs/adr/0010-export-schema-versioning.md).
pub const SCHEMA_VERSION: u32 = 1;

/// The Kul language version this `kul-core` build implements. Surfaced in
/// the success envelope as `kul:` so consumers can warn the user when the
/// source predates a feature they rely on. Distinct from `crate::VERSION`,
/// which is the implementation version of this crate.
pub const LANGUAGE_VERSION: &str = "0.1";

/// Output format for [`export`].
///
/// `Deserialize` accepts the lowercase wire form (`"json"`, `"cytoscape"`)
/// so JS-side consumers and CLI flag parsing share one vocabulary. See
/// [`ExportOptions`] for the camelCase wrapper that `kul-wasm`'s
/// `exportGraph` uses on its options input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// The canonical kinship-native JSON shape — three flat collections
    /// (`persons`, `marriages`, `parenthood_links`) mirroring the
    /// language primitives. See [`spec/16-export-schema.md`](../../../spec/16-export-schema.md).
    #[default]
    Json,
    /// The Cytoscape JSON shape — `nodes` + `edges`, with marriages
    /// promoted to first-class nodes. Loadable into Cytoscape Desktop,
    /// Cytoscape.js, Sigma.js, vis-network, and similar generic graph
    /// tooling without modification. See [`cytoscape`].
    Cytoscape,
}

/// Caller-tunable knobs for [`export`]. Defaults are the most common path.
///
/// `Deserialize` is camelCase and field-level `default` so a JS-side caller
/// can pass `{}`, `{ withPositions: true }`, or `{ format: "cytoscape" }`
/// and the omitted fields fall back to [`ExportOptions::default`]. The
/// `kul-wasm` `exportGraph` bridge uses this directly.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(from_wasm_abi))]
#[serde(default, rename_all = "camelCase")]
pub struct ExportOptions {
    pub format: ExportFormat,
    /// When `true`, every exported entity carries a `span: [byte_start,
    /// byte_end]` field pointing back to its declaration in the source.
    /// Default `false` keeps the envelope compact; opt in when the
    /// consumer needs to map a click on a graph node back to a source
    /// location ("highlight Alice's declaration").
    pub with_positions: bool,
}

/// The export envelope returned by [`export`]. Either a success payload
/// carrying the graph, or a failure payload carrying the diagnostic list.
///
/// Serialized untagged: serde picks the variant by structure. Both variants
/// carry an `ok` boolean so consumers can discriminate without inspecting
/// other fields.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
#[serde(untagged, rename_all = "camelCase")]
pub enum ExportEnvelope {
    Success(SuccessEnvelope),
    Failure(FailureEnvelope),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct SuccessEnvelope {
    /// Always `true`. Consumer-facing discriminator.
    pub ok: bool,
    /// Schema version this envelope conforms to. See [`SCHEMA_VERSION`].
    pub schema: u32,
    /// Kul language version of the source document, sourced from the
    /// project manifest's `kul:` field (`kul.yml`).
    pub kul: String,
    /// The exported graph. Either the kinship-native shape (the canonical
    /// foundation) or a derived shape such as Cytoscape, depending on
    /// [`ExportOptions::format`]. Untagged in the JSON: the consumer
    /// knows which shape to expect from the format they requested.
    pub graph: GraphPayload,
}

/// A graph payload inside a [`SuccessEnvelope`].
///
/// Untagged at the wire level: the JSON looks identical to whichever
/// inner shape was chosen (kinship-native objects with `persons` /
/// `marriages` / `parenthood_links`, or cytoscape objects with `nodes` /
/// `edges`). Consumers know which to expect based on the `--format` they
/// asked for; the envelope's `schema` is the same integer regardless of
/// shape because both shapes are projections of the same underlying data.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(untagged, rename_all = "camelCase")]
pub enum GraphPayload {
    /// The kinship-native graph: three flat collections.
    Native(ExportedGraph),
    /// The Cytoscape graph: `nodes` + `edges` with marriage-as-node
    /// modeling. Derived from [`Native`].
    Cytoscape(cytoscape::CytoscapeGraph),
}

impl GraphPayload {
    /// Borrow as the kinship-native graph, or `None` if this payload is
    /// in another shape. Test helper.
    pub fn as_native(&self) -> Option<&ExportedGraph> {
        match self {
            GraphPayload::Native(g) => Some(g),
            _ => None,
        }
    }

    /// Borrow as the Cytoscape graph, or `None` if this payload is in
    /// another shape. Test helper.
    pub fn as_cytoscape(&self) -> Option<&cytoscape::CytoscapeGraph> {
        match self {
            GraphPayload::Cytoscape(g) => Some(g),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct FailureEnvelope {
    /// Always `false`. Consumer-facing discriminator.
    pub ok: bool,
    /// Every diagnostic the validator produced — errors, warnings, and
    /// notes alike — so the consumer sees the full picture of why export
    /// refused.
    pub diagnostics: Vec<ExportedDiagnostic>,
}

impl ExportEnvelope {
    pub fn is_ok(&self) -> bool {
        matches!(self, ExportEnvelope::Success(_))
    }
}

/// The kinship-native graph: three flat collections.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct ExportedGraph {
    pub persons: Vec<ExportedPerson>,
    pub marriages: Vec<ExportedMarriage>,
    pub parenthood_links: Vec<ExportedParenthoodLink>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct ExportedPerson {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<String>,
    pub gender: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub born: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub died: Option<ExportedDate>,
    /// `[byte_start, byte_end]` covering the source-level statement.
    /// Present only when `ExportOptions::with_positions` was `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<[usize; 2]>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct ExportedMarriage {
    pub id: String,
    /// The two spouse ids, in declaration order. Both ids resolve to a
    /// `person` in `persons` (the failure envelope would have fired
    /// otherwise).
    pub spouses: [String; 2],
    pub start: ExportedDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
    /// `[byte_start, byte_end]` covering the source-level statement.
    /// Present only when `ExportOptions::with_positions` was `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<[usize; 2]>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct ExportedParenthoodLink {
    pub marriage_id: String,
    pub child_id: String,
    /// `"biological"` or `"adoptive"`. New kinds (e.g. surrogacy) would
    /// land additively per [ADR-0010](../../../docs/adr/0010-export-schema-versioning.md).
    pub kind: &'static str,
    /// `start:` of an adoption. Always absent for biological links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<ExportedDate>,
    /// `end:` of an adoption. Always absent for biological links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ExportedDate>,
    /// `[byte_start, byte_end]` covering the source-level `birth` or
    /// `adoption` sub-statement. Present only when
    /// `ExportOptions::with_positions` was `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<[usize; 2]>,
}

/// A date as projected into the envelope. Splits the source `~YYYY[-MM[-DD]]`
/// form into `value` (no circa marker), `precision` (year / month / day),
/// and `circa` (the `~` flag) so consumers don't have to re-parse strings.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct ExportedDate {
    pub value: String,
    pub precision: &'static str,
    pub circa: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct ExportedDiagnostic {
    pub code: String,
    pub severity: &'static str,
    pub message: String,
    /// `None` for unanchored diagnostics (e.g. `KUL-M01`); the message
    /// carries the would-be location in that case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<ExportedSpan>,
    pub related: Vec<ExportedRelated>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct ExportedRelated {
    pub label: String,
    #[serde(flatten)]
    pub span: ExportedSpan,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct ExportedSpan {
    /// Canonical name of the file this span anchors into (the
    /// `InputFile.name` the toolchain originally fed in, or the
    /// manifest's `manifest_name` for `KUL-Mxx` codes).
    pub file: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line: usize,
    pub column: usize,
}

/// Project every diagnostic in a [`CheckResult`] into the wire-shape
/// [`ExportedDiagnostic`] used by the failure envelope.
///
/// Each diagnostic's primary [`FileSpan`] is rendered with its file's
/// own [`SourceMap`] (built lazily and cached for the duration of the
/// call). The `kul-wasm` `check` bridge calls this to expose diagnostics
/// over the JS surface without reimplementing diagnostic-to-JSON walking
/// or per-file `SourceMap` construction.
pub fn export_diagnostics(check: &CheckResult) -> Vec<ExportedDiagnostic> {
    let document = check.document();
    let mut maps = SourceMapCache::new(document);
    check
        .diagnostics
        .iter()
        .map(|d| exported_diagnostic(d, document, &mut maps))
        .collect()
}

/// Project a [`CheckResult`] into an [`ExportEnvelope`].
///
/// Strict on errors: any error-severity diagnostic returns a failure
/// envelope carrying the full diagnostic list. Warnings do not block;
/// they are not surfaced in the success envelope today (additive — a
/// future schema bump may include them).
pub fn export(check: &CheckResult, options: ExportOptions) -> ExportEnvelope {
    if check.has_errors() {
        let diagnostics = export_diagnostics(check);
        return ExportEnvelope::Failure(FailureEnvelope {
            ok: false,
            diagnostics,
        });
    }
    let resolved = check.resolved();
    let native = build_graph(resolved, &options);
    let graph = match options.format {
        ExportFormat::Json => GraphPayload::Native(native),
        ExportFormat::Cytoscape => GraphPayload::Cytoscape(cytoscape::to_cytoscape(&native)),
    };
    ExportEnvelope::Success(SuccessEnvelope {
        ok: true,
        schema: SCHEMA_VERSION,
        kul: check.manifest.kul_version.clone(),
        graph,
    })
}

/// Lazily-built per-file [`SourceMap`] cache. The diagnostic list may
/// span the manifest plus any number of `.kul` files; each file gets at
/// most one [`SourceMap`] built on first need.
struct SourceMapCache<'a> {
    document: &'a Document,
    maps: HashMap<FileId, SourceMap>,
}

impl<'a> SourceMapCache<'a> {
    fn new(document: &'a Document) -> Self {
        Self {
            document,
            maps: HashMap::new(),
        }
    }

    fn for_file(&mut self, file: FileId) -> Option<&SourceMap> {
        if !self.maps.contains_key(&file) {
            let source = self.document.source_of(file)?;
            self.maps.insert(file, SourceMap::new(source));
        }
        self.maps.get(&file)
    }
}

fn build_graph(resolved: &ResolvedDocument, options: &ExportOptions) -> ExportedGraph {
    let persons = resolved
        .persons()
        .map(|p| exported_person(p, options))
        .collect();
    let marriages = resolved
        .marriages()
        .map(|m| ExportedMarriage {
            id: m.id.name.clone(),
            spouses: [m.spouse_a.name.clone(), m.spouse_b.name.clone()],
            start: exported_date(m.start().expect("R03 ensures marriage.start is present")),
            end: m.end().map(exported_date),
            end_reason: m.end_reason().map(|er| end_reason_str(&er.value)),
            span: span_if(options, m.span),
        })
        .collect();
    let parenthood_links = build_parenthood_links(resolved, options);
    ExportedGraph {
        persons,
        marriages,
        parenthood_links,
    }
}

fn build_parenthood_links(
    resolved: &ResolvedDocument,
    options: &ExportOptions,
) -> Vec<ExportedParenthoodLink> {
    let capacity = resolved
        .persons()
        .map(|p| usize::from(p.birth.is_some()) + p.adoptions.len())
        .sum();
    let mut out: Vec<ExportedParenthoodLink> = Vec::with_capacity(capacity);
    for p in resolved.persons() {
        if let Some(birth) = &p.birth {
            out.push(ExportedParenthoodLink {
                marriage_id: birth.marriage_ref.name.clone(),
                child_id: p.id.name.clone(),
                kind: "biological",
                start: None,
                end: None,
                span: span_if(options, birth.span),
            });
        }
        for adoption in &p.adoptions {
            out.push(ExportedParenthoodLink {
                marriage_id: adoption.marriage_ref.name.clone(),
                child_id: p.id.name.clone(),
                kind: "adoptive",
                start: adoption.start().map(exported_date),
                end: adoption.end().map(exported_date),
                span: span_if(options, adoption.span),
            });
        }
    }
    out
}

fn exported_person(p: &PersonStmt, options: &ExportOptions) -> ExportedPerson {
    ExportedPerson {
        id: p.id.name.clone(),
        name: p
            .name()
            .expect("R03 ensures person.name is present")
            .value
            .clone(),
        family: p.family().map(|s| s.value.clone()),
        given: p.given().map(|s| s.value.clone()),
        gender: gender_str(
            p.gender()
                .expect("R03 ensures person.gender is present")
                .value,
        ),
        born: p.born().map(exported_date),
        died: p.died().map(exported_date),
        span: span_if(options, p.span),
    }
}

fn span_if(options: &ExportOptions, span: ByteSpan) -> Option<[usize; 2]> {
    options.with_positions.then_some([span.start, span.end])
}

fn exported_date(d: &DateLit) -> ExportedDate {
    let (value, precision) = match (d.month, d.day) {
        (Some(m), Some(day)) => (format!("{:04}-{:02}-{:02}", d.year, m, day), "day"),
        (Some(m), None) => (format!("{:04}-{:02}", d.year, m), "month"),
        (None, _) => (format!("{:04}", d.year), "year"),
    };
    ExportedDate {
        value,
        precision,
        circa: d.circa,
    }
}

fn gender_str(g: Gender) -> &'static str {
    match g {
        Gender::Male => "male",
        Gender::Female => "female",
        Gender::Other => "other",
    }
}

fn end_reason_str(er: &EndReason) -> String {
    match er {
        EndReason::Divorce => "divorce".to_string(),
        EndReason::Unknown(s) => s.clone(),
    }
}

fn exported_diagnostic(
    d: &Diagnostic,
    document: &Document,
    maps: &mut SourceMapCache<'_>,
) -> ExportedDiagnostic {
    ExportedDiagnostic {
        code: d.code.to_string(),
        severity: severity_str(d.severity),
        message: d.message.clone(),
        primary: d.primary.and_then(|p| exported_span(p, document, maps)),
        related: d
            .related
            .iter()
            .filter_map(|r| {
                let span = exported_span(r.span, document, maps)?;
                Some(ExportedRelated {
                    label: r.label.clone(),
                    span,
                })
            })
            .collect(),
    }
}

fn exported_span(
    fs: FileSpan,
    document: &Document,
    maps: &mut SourceMapCache<'_>,
) -> Option<ExportedSpan> {
    let map = maps.for_file(fs.file)?;
    let lc = map.line_col(fs.span.start);
    let name = document.name_of(fs.file).unwrap_or("").to_string();
    Some(ExportedSpan {
        file: name,
        byte_start: fs.span.start,
        byte_end: fs.span.end,
        line: lc.line,
        column: lc.column,
    })
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export_source(source: &str) -> ExportEnvelope {
        let inputs = vec![crate::ast::InputFile::new("test.kul", source)];
        let check = crate::check_with_manifest(
            "kul.yml",
            "",
            &crate::manifest::Manifest::default(),
            &inputs,
        );
        export(&check, ExportOptions::default())
    }

    fn native_graph(env: ExportEnvelope) -> ExportedGraph {
        let ExportEnvelope::Success(s) = env else {
            panic!("expected success envelope");
        };
        s.graph
            .as_native()
            .cloned()
            .expect("expected native graph payload")
    }

    #[test]
    fn empty_document_succeeds_with_empty_collections() {
        let env = export_source("");
        let ExportEnvelope::Success(s) = env.clone() else {
            panic!("expected success");
        };
        assert_eq!(s.schema, SCHEMA_VERSION);
        assert_eq!(s.kul, LANGUAGE_VERSION);
        let g = native_graph(env);
        assert!(g.persons.is_empty());
        assert!(g.marriages.is_empty());
        assert!(g.parenthood_links.is_empty());
    }

    #[test]
    fn manifest_version_propagates_to_envelope_kul_field() {
        let manifest = crate::manifest::Manifest {
            kul_version: "0.1".to_string(),
        };
        let inputs = vec![crate::ast::InputFile::new(
            "test.kul",
            "person alice name:\"Alice\" gender:female\n",
        )];
        let check = crate::check_with_manifest("kul.yml", "", &manifest, &inputs);
        let env = export(&check, ExportOptions::default());
        let ExportEnvelope::Success(s) = env else {
            panic!("expected success");
        };
        assert_eq!(s.kul, "0.1");
    }

    #[test]
    fn errors_block_with_failure_envelope() {
        let env = export_source("person alice gender:female\n"); // missing name → R03
        let ExportEnvelope::Failure(f) = env else {
            panic!("expected failure");
        };
        assert!(!f.ok);
        assert!(f.diagnostics.iter().any(|d| d.code == "KUL-R03"));
        assert!(
            f.diagnostics
                .iter()
                .any(|d| d.primary.as_ref().map(|s| s.line >= 1).unwrap_or(false))
        );
    }

    #[test]
    fn polygamy_emits_two_marriages_for_one_person() {
        let src = "\
person alice name:\"Alice\" gender:female born:1950
person bob name:\"Bob\" gender:male born:1948
person carol name:\"Carol\" gender:female born:1952
marriage m1 alice bob start:1972
marriage m2 alice carol start:1980
";
        let g = native_graph(export_source(src));
        let alice_marriages = g
            .marriages
            .iter()
            .filter(|m| m.spouses.iter().any(|sp| sp == "alice"))
            .count();
        assert_eq!(alice_marriages, 2);
    }

    #[test]
    fn child_with_birth_and_adoption_emits_two_parenthood_links() {
        let src = "\
person a name:\"A\" gender:female born:1950
person b name:\"B\" gender:male born:1948
person c name:\"C\" gender:female born:1952
person d name:\"D\" gender:male born:1950
person kid name:\"K\" gender:other born:1980
  birth m1
  adoption m2 start:1990
marriage m1 a b start:1972
marriage m2 c d start:1971
";
        let g = native_graph(export_source(src));
        let kid_links: Vec<_> = g
            .parenthood_links
            .iter()
            .filter(|l| l.child_id == "kid")
            .collect();
        assert_eq!(kid_links.len(), 2);
        assert_eq!(kid_links[0].kind, "biological");
        assert_eq!(kid_links[0].marriage_id, "m1");
        assert_eq!(kid_links[1].kind, "adoptive");
        assert_eq!(kid_links[1].marriage_id, "m2");
        assert!(kid_links[1].start.is_some());
    }

    #[test]
    fn date_precision_and_circa_round_trip() {
        let src = "\
person a name:\"A\" gender:female born:1980
person b name:\"B\" gender:male born:1980-03
person c name:\"C\" gender:other born:1980-03-15
person d name:\"D\" gender:female born:~1980
";
        let g = native_graph(export_source(src));
        let by_id = |id: &str| {
            g.persons
                .iter()
                .find(|p| p.id == id)
                .unwrap()
                .born
                .clone()
                .unwrap()
        };
        assert_eq!(by_id("a").value, "1980");
        assert_eq!(by_id("a").precision, "year");
        assert!(!by_id("a").circa);
        assert_eq!(by_id("b").value, "1980-03");
        assert_eq!(by_id("b").precision, "month");
        assert_eq!(by_id("c").value, "1980-03-15");
        assert_eq!(by_id("c").precision, "day");
        assert_eq!(by_id("d").value, "1980");
        assert!(by_id("d").circa);
    }

    #[test]
    fn cytoscape_format_returns_cytoscape_payload() {
        let src = "person alice name:\"A\" gender:female\n";
        let inputs = vec![crate::ast::InputFile::new("test.kul", src)];
        let check = crate::check_with_manifest(
            "kul.yml",
            "",
            &crate::manifest::Manifest::default(),
            &inputs,
        );
        let env = export(
            &check,
            ExportOptions {
                format: ExportFormat::Cytoscape,
                ..ExportOptions::default()
            },
        );
        let ExportEnvelope::Success(s) = env else {
            panic!("expected success");
        };
        assert!(
            s.graph.as_cytoscape().is_some(),
            "cytoscape format should produce cytoscape payload"
        );
    }

    #[test]
    fn export_options_deserialize_empty_object_yields_defaults() {
        let opts: ExportOptions = serde_json::from_str("{}").unwrap();
        assert_eq!(opts.format, ExportFormat::Json);
        assert!(!opts.with_positions);
    }

    #[test]
    fn export_options_deserialize_explicit_camelcase_fields() {
        let opts: ExportOptions =
            serde_json::from_str(r#"{"format":"cytoscape","withPositions":true}"#).unwrap();
        assert_eq!(opts.format, ExportFormat::Cytoscape);
        assert!(opts.with_positions);
    }

    #[test]
    fn export_options_deserialize_partial_object_falls_back_to_defaults() {
        let opts: ExportOptions = serde_json::from_str(r#"{"withPositions":true}"#).unwrap();
        assert_eq!(opts.format, ExportFormat::Json);
        assert!(opts.with_positions);
    }

    #[test]
    fn export_format_rejects_unknown_variant() {
        let err = serde_json::from_str::<ExportFormat>(r#""graphviz""#).unwrap_err();
        assert!(
            err.to_string().contains("graphviz"),
            "error should mention the unknown variant: {err}"
        );
    }
}
