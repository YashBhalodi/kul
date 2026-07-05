//! `kul query` subcommand — id → detail lookups over the CWD Kul project.
//!
//! Two verbs: `kul query person <id>` and `kul query marriage <id>`. Like
//! `kul export`, the command runs only against a project that passes its
//! checks (strict-on-errors); a failing project prints diagnostics and
//! exits nonzero.
//!
//! `--format json` emits the exact [`QueryEnvelope`] serialization the WASM
//! surface returns — this path is the epic's contract-snapshot harness.
//! `--format human` renders the entity's recorded fields in a readable,
//! terminology-neutral layout; presentation is owned by the CLI, not a
//! third contract shape.
//!
//! Not-found is honest: the JSON `result` is `null` (the ok envelope is
//! still the contract answer, on stdout), while a diagnostic naming the id
//! goes to stderr and the exit code is nonzero.

use std::io::{self, Write};
use std::process::ExitCode;

use serde::Serialize;

use kul_core::CheckResult;
use kul_core::export::{ExportedDate, ExportedMarriage, ExportedPerson};
use kul_core::query::{
    Classification, EdgeNature, LinealRole, Member, Query, QueryEnvelope, QueryResult, Seniority,
    Side, kin_query, marriage_lookup, person_lookup,
};

use crate::OutputFormat;
use crate::commands::diag;
use crate::commands::project::load_and_check;

/// Which detail lookup to run.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Verb {
    Person,
    Marriage,
}

impl Verb {
    /// Noun used in the not-found diagnostic (`no <noun> with id ...`).
    fn noun(self) -> &'static str {
        match self {
            Verb::Person => "person",
            Verb::Marriage => "marriage",
        }
    }
}

pub struct Options {
    pub verb: Verb,
    pub id: String,
    pub format: OutputFormat,
}

pub fn run(opts: Options) -> ExitCode {
    let (_project, check) = match load_and_check() {
        Ok(x) => x,
        Err(code) => return code,
    };
    match opts.verb {
        Verb::Person => {
            let envelope = person_lookup(&check, &opts.id);
            finish(&envelope, &opts, &check, render_person_human)
        }
        Verb::Marriage => {
            let envelope = marriage_lookup(&check, &opts.id);
            finish(&envelope, &opts, &check, render_marriage_human)
        }
    }
}

/// Emit one lookup envelope in the requested format and choose the exit
/// code. Shared by both verbs — the payload type `E` is the export shape,
/// `render_human` turns a found entity into its human layout.
fn finish<E: Serialize>(
    envelope: &QueryEnvelope<Option<E>>,
    opts: &Options,
    check: &CheckResult,
    render_human: impl Fn(&E) -> String,
) -> ExitCode {
    match opts.format {
        OutputFormat::Json => finish_json(envelope, opts),
        OutputFormat::Human => finish_human(envelope, opts, check, render_human),
    }
}

fn finish_json<E: Serialize>(envelope: &QueryEnvelope<Option<E>>, opts: &Options) -> ExitCode {
    // The envelope IS the contract answer, even for not-found (null result)
    // and failing-check (error arm) cases — always to stdout.
    let json = serde_json::to_string(envelope).expect("serialize query envelope");
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = writeln!(out, "{json}") {
        eprintln!("kul: failed to write query envelope: {err}");
        return ExitCode::from(1);
    }
    match envelope {
        QueryEnvelope::Error(_) => ExitCode::from(1),
        QueryEnvelope::Ok(ok) => match &ok.result {
            Some(_) => ExitCode::SUCCESS,
            None => {
                not_found(opts);
                ExitCode::from(1)
            }
        },
    }
}

fn finish_human<E: Serialize>(
    envelope: &QueryEnvelope<Option<E>>,
    opts: &Options,
    check: &CheckResult,
    render_human: impl Fn(&E) -> String,
) -> ExitCode {
    match envelope {
        QueryEnvelope::Error(_) => {
            // Same load-and-check gate as export: diagnostics to stderr,
            // nonzero exit — never a partial answer.
            diag::render_human(check, false);
            ExitCode::from(1)
        }
        QueryEnvelope::Ok(ok) => match &ok.result {
            Some(entity) => {
                print!("{}", render_human(entity));
                ExitCode::SUCCESS
            }
            None => {
                not_found(opts);
                ExitCode::from(1)
            }
        },
    }
}

/// Print the not-found diagnostic to stderr (e.g. ``no person with id `x7` ``).
fn not_found(opts: &Options) {
    eprintln!("kul: no {} with id `{}`", opts.verb.noun(), opts.id);
}

/// Render an [`ExportedDate`] with its circa marker (`~1925`, `1980-06`).
fn fmt_date(d: &ExportedDate) -> String {
    if d.circa {
        format!("~{}", d.value)
    } else {
        d.value.clone()
    }
}

fn render_person_human(p: &ExportedPerson) -> String {
    let mut out = format!("person {}\n", p.id);
    let mut line = |label: &str, value: &str| out.push_str(&format!("  {label:<9}{value}\n"));
    line("name:", &p.name);
    if let Some(family) = &p.family {
        line("family:", family);
    }
    if let Some(given) = &p.given {
        line("given:", given);
    }
    line("gender:", p.gender);
    if let Some(born) = &p.born {
        line("born:", &fmt_date(born));
    }
    if let Some(died) = &p.died {
        line("died:", &fmt_date(died));
    }
    out
}

fn render_marriage_human(m: &ExportedMarriage) -> String {
    let mut out = format!("marriage {}\n", m.id);
    let mut line = |label: &str, value: &str| out.push_str(&format!("  {label:<10}{value}\n"));
    line("spouses:", &format!("{}, {}", m.spouses[0], m.spouses[1]));
    if let Some(start) = &m.start {
        line("start:", &fmt_date(start));
    }
    if let Some(end) = &m.end {
        line("end:", &fmt_date(end));
    }
    if let Some(reason) = &m.end_reason {
        line("reason:", reason);
    }
    out
}

// ---- Kin-set queries (`kul query kin <anchor> <relation>`) ----

/// Options for a `kul query kin` run: the anchor + the desugared [`Query`]
/// value (built by the arg parser, so the CLI has no query semantics of its
/// own) + the output format.
pub struct KinOptions {
    pub anchor: String,
    pub query: Query,
    pub format: OutputFormat,
}

pub fn run_kin(opts: KinOptions) -> ExitCode {
    let (_project, check) = match load_and_check() {
        Ok(x) => x,
        Err(code) => return code,
    };
    // One evaluation path: the same envelope the WASM `queryKin` surface
    // returns, so `--format json` bytes are byte-identical.
    let envelope = kin_query(&check, &opts.query);
    match opts.format {
        OutputFormat::Json => finish_kin_json(&envelope),
        OutputFormat::Human => finish_kin_human(&envelope, &opts, &check),
    }
}

fn finish_kin_json(envelope: &QueryEnvelope<QueryResult>) -> ExitCode {
    // The envelope IS the contract answer for every outcome (empty set, bad
    // anchor, failing project) — always stdout, and it already carries the
    // diagnostic for the error arms. No stderr echo, so a failing-project
    // envelope is never mislabelled as a bad anchor.
    // Infallible: the envelope is built from owned, well-formed data.
    let json = serde_json::to_string(envelope).expect("serialize kin envelope");
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = writeln!(out, "{json}") {
        eprintln!("kul: failed to write query envelope: {err}");
        return ExitCode::from(1);
    }
    match envelope {
        QueryEnvelope::Error(_) => ExitCode::from(1),
        // An empty set is a complete answer (exit 0).
        QueryEnvelope::Ok(_) => ExitCode::SUCCESS,
    }
}

fn finish_kin_human(
    envelope: &QueryEnvelope<QueryResult>,
    opts: &KinOptions,
    check: &CheckResult,
) -> ExitCode {
    match envelope {
        QueryEnvelope::Error(_) => {
            if check.has_errors() {
                // Load-and-check gate: render the project's diagnostics.
                diag::render_human(check, false);
            } else {
                // A clean project with an error arm can only be a bad anchor.
                not_found_anchor(&opts.anchor);
            }
            ExitCode::from(1)
        }
        QueryEnvelope::Ok(ok) => {
            let QueryResult::Members { members } = &ok.result;
            // Empty set → print nothing, exit 0.
            for member in members {
                print!("{}", render_member_human(member, check));
            }
            ExitCode::SUCCESS
        }
    }
}

/// The bad-anchor diagnostic to stderr (mirrors the lookup not-found line).
fn not_found_anchor(anchor: &str) {
    eprintln!("kul: no person with id `{anchor}`");
}

/// One member as a terminology-neutral block: id + display name, then the
/// descriptor's structured facts. **Never a kinship word** — rendering
/// "grandmother" is the future terminology layer's job, not the CLI's.
fn render_member_human(member: &Member, check: &CheckResult) -> String {
    let name = check
        .resolved()
        .person(&member.person_id)
        .map(|p| p.display_name().to_string())
        .unwrap_or_else(|| member.person_id.clone());
    format!(
        "{}  {}\n  {}\n",
        member.person_id,
        name,
        descriptor_facts(&member.descriptor)
    )
}

/// The descriptor's structured facts as a middot-separated line, e.g.
/// `lineal ancestor · 2 generations · blood · maternal · elder`. Every token
/// is a descriptor field value, not a kinship term.
fn descriptor_facts(d: &kul_core::query::RelationshipDescriptor) -> String {
    let classification = match &d.classification {
        Classification::SelfRel => "self".to_string(),
        Classification::Lineal { role, generations } => {
            let role = match role {
                LinealRole::Ancestor => "ancestor",
                LinealRole::Descendant => "descendant",
            };
            let unit = if *generations == 1 {
                "generation"
            } else {
                "generations"
            };
            format!("lineal {role} · {generations} {unit}")
        }
        Classification::Collateral {
            up,
            down,
            cousin_degree,
            removed,
        } => format!(
            "collateral up {up} down {down} · cousin degree {cousin_degree} · removed {removed}"
        ),
    };
    let edge = match d.edge_nature {
        EdgeNature::Blood => "blood",
        EdgeNature::Adoptive => "adoptive",
    };
    format!(
        "{classification} · {edge} · side {} · {}",
        side_word(d.side),
        seniority_word(d.seniority),
    )
}

fn side_word(side: Side) -> &'static str {
    match side {
        Side::Maternal => "maternal",
        Side::Paternal => "paternal",
        Side::Other => "other",
        Side::Both => "both",
        Side::NotApplicable => "n/a",
    }
}

fn seniority_word(seniority: Seniority) -> &'static str {
    match seniority {
        Seniority::Elder => "elder",
        Seniority::Younger => "younger",
        Seniority::Unknown => "unknown",
        Seniority::NotApplicable => "n/a",
    }
}
