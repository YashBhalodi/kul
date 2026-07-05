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
use kul_core::query::{QueryEnvelope, marriage_lookup, person_lookup};

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
