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
use kul_core::ast::PersonStmt;
use kul_core::export::{ExportedDate, ExportedMarriage, ExportedPerson};
use kul_core::query::{
    Affinity, Classification, EdgeNature, EmptyReason, HopEdge, LinealRole, MarriageStatus, Member,
    PathHop, PersonField, Predicate, Query, QueryEnvelope, QueryResult, RelationshipDescriptor,
    ResolveConfig, ResolveResult, Seniority, Sharing, Side, SortDirection, SortSpec, kin_query,
    marriage_lookup, person_lookup, query_envelope, resolve_relationship,
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
        // The json envelope is the contract answer for every outcome (empty
        // set, bad anchor, failing project) — always stdout, carrying the
        // diagnostic on the error arms. Shared with the persons path so the
        // `--format json` bytes are byte-identical across surfaces.
        OutputFormat::Json => finish_query_json(&envelope),
        OutputFormat::Human => finish_kin_human(&envelope, &opts, &check),
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
            match &ok.result {
                // Empty set → print nothing, exit 0.
                QueryResult::Members { members } => {
                    for member in members {
                        print!("{}", render_member_human(member, check));
                    }
                }
                // A `--count` kin query: just the integer.
                QueryResult::Count { count } => println!("{count}"),
                // `kinOf` never projects `personIds` (that is the `allPersons`
                // shape), but stay total rather than panic.
                QueryResult::PersonIds { person_ids } => {
                    for id in person_ids {
                        println!("{id}");
                    }
                }
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
    let mut block = format!(
        "{}  {}\n  {}\n",
        member.person_id,
        name,
        descriptor_facts(&member.descriptor)
    );
    // Marriage hops on the path render on their own lines with the marriage id,
    // status, and end reason — the affinal backbone the descriptor walked.
    for hop in &member.descriptor.path {
        if let PathHop::Across {
            marriage,
            status,
            end_reason,
            ..
        } = hop
        {
            block.push_str(&format!("  across {marriage} · {}", status_word(*status)));
            if let Some(reason) = end_reason {
                block.push_str(&format!(" · {reason}"));
            }
            block.push('\n');
        }
    }
    block
}

/// The marriage-status token for an `across` hop.
fn status_word(status: MarriageStatus) -> &'static str {
    match status {
        MarriageStatus::Ongoing => "ongoing",
        MarriageStatus::Ended => "ended",
    }
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
        } => format!("collateral {up}/{down} · cousinDegree {cousin_degree}, removed {removed}"),
    };
    let edge = match d.edge_nature {
        EdgeNature::Blood => "blood",
        EdgeNature::Adoptive => "adoptive",
    };
    let mut facts = format!("{classification} · {edge}");
    // Affinity is shown only when the path runs through marriage; a blood path
    // is always `blood` and would only clutter the lineal / collateral line.
    if let Some(word) = affinity_word(d.affinity) {
        facts.push_str(&format!(" · {word}"));
    }
    // `sharing` and `apexSeniority` apply only at a sibling junction; skip the
    // `notApplicable` cases so lineal output stays a clean line.
    if let Some(word) = sharing_word(d.sharing) {
        facts.push_str(&format!(" · {word}"));
    }
    facts.push_str(&format!(" · side {}", side_word(d.side)));
    facts.push_str(&format!(" · {}", seniority_word(d.seniority)));
    if let Some(word) = apex_seniority_word(d.apex_seniority) {
        facts.push_str(&format!(" · apex {word}"));
    }
    facts
}

/// The sharing token, or `None` for `notApplicable` (lineal / self paths,
/// which carry no sibling junction).
fn sharing_word(sharing: Sharing) -> Option<&'static str> {
    match sharing {
        Sharing::Full => Some("full"),
        Sharing::Half => Some("half"),
        Sharing::NotApplicable => None,
    }
}

/// The apex-seniority token, or `None` for `notApplicable` (no sibling
/// junction on the path).
fn apex_seniority_word(seniority: Seniority) -> Option<&'static str> {
    match seniority {
        Seniority::NotApplicable => None,
        other => Some(seniority_word(other)),
    }
}

/// The affinity token, or `None` for `blood` (a non-affinal path carries no
/// marriage hop, so the token would be noise).
fn affinity_word(affinity: Affinity) -> Option<&'static str> {
    match affinity {
        Affinity::Blood => None,
        Affinity::Step => Some("step"),
        Affinity::InLaw => Some("inLaw"),
    }
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

// ---- Attribute-filter queries (`kul query persons`) ----

/// Options for a `kul query persons` run: the desugared [`Query`] value (an
/// `allPersons` source plus the parsed filter flags) + the output format.
pub struct PersonsOptions {
    pub query: Query,
    pub format: OutputFormat,
}

pub fn run_persons(opts: PersonsOptions) -> ExitCode {
    let (_project, check) = match load_and_check() {
        Ok(x) => x,
        Err(code) => return code,
    };
    // One evaluation path: the same envelope the WASM `runQuery` surface
    // returns, so `--format json` bytes are byte-identical.
    let envelope = query_envelope(&check, &opts.query);
    match opts.format {
        OutputFormat::Json => finish_query_json(&envelope),
        OutputFormat::Human => finish_persons_human(&envelope, &opts, &check),
    }
}

/// Serialize a query envelope to stdout (the contract answer for every
/// outcome). Error arm → exit 1; ok arm (including an empty set) → exit 0.
/// Shared by the kin and persons json paths.
fn finish_query_json(envelope: &QueryEnvelope<QueryResult>) -> ExitCode {
    let json = serde_json::to_string(envelope).expect("serialize query envelope");
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = writeln!(out, "{json}") {
        eprintln!("kul: failed to write query envelope: {err}");
        return ExitCode::from(1);
    }
    match envelope {
        QueryEnvelope::Error(_) => ExitCode::from(1),
        QueryEnvelope::Ok(_) => ExitCode::SUCCESS,
    }
}

fn finish_persons_human(
    envelope: &QueryEnvelope<QueryResult>,
    opts: &PersonsOptions,
    check: &CheckResult,
) -> ExitCode {
    match envelope {
        QueryEnvelope::Error(err) => {
            if check.has_errors() {
                // Load-and-check gate: render the project's diagnostics.
                diag::render_human(check, false);
            } else {
                // A clean project with an error arm can only be a malformed
                // predicate; the synthesized diagnostic already says which.
                for d in &err.diagnostics {
                    eprintln!("kul: {}", d.message);
                }
            }
            ExitCode::from(1)
        }
        QueryEnvelope::Ok(ok) => {
            match &ok.result {
                // A `--count` query: just the integer.
                QueryResult::Count { count } => println!("{count}"),
                // The `allPersons` set: one line per person (id, display name,
                // and the sort field's value when sorting). Empty → nothing.
                QueryResult::PersonIds { person_ids } => {
                    let sort_field = opts.query.sort.map(|s| s.field);
                    for id in person_ids {
                        print!("{}", render_person_line(id, sort_field, check));
                    }
                }
                // `allPersons` never projects `members` (no descriptor), but
                // stay total rather than panic.
                QueryResult::Members { members } => {
                    for member in members {
                        print!("{}", render_member_human(member, check));
                    }
                }
            }
            ExitCode::SUCCESS
        }
    }
}

/// One `persons` line: `id  display-name`, plus `  field:value` for the sort
/// field when sorting (a missing sort value renders `—`). Terminology-neutral.
fn render_person_line(id: &str, sort_field: Option<PersonField>, check: &CheckResult) -> String {
    let person = check.resolved().person(id);
    let name = person
        .map(|p| p.display_name().to_string())
        .unwrap_or_else(|| id.to_string());
    let mut line = format!("{id}  {name}");
    // The sort field's value (skip `id` — it already leads the line).
    if let Some(field) = sort_field.filter(|f| *f != PersonField::Id) {
        let value = person
            .and_then(|p| field_value(p, field))
            .unwrap_or_else(|| "—".to_string());
        line.push_str(&format!("  {}:{value}", field.as_str()));
    }
    line.push('\n');
    line
}

/// The person's value for a field, as a display string (`None` when absent).
/// Dates render canonically (`~1925`, `1950-06`); gender to its token.
fn field_value(person: &PersonStmt, field: PersonField) -> Option<String> {
    match field {
        PersonField::Id => Some(person.id.name.clone()),
        PersonField::Name => person.name().map(|v| v.value.clone()),
        PersonField::Family => person.family().map(|v| v.value.clone()),
        PersonField::Given => person.given().map(|v| v.value.clone()),
        PersonField::Gender => person.gender().map(|g| {
            use kul_core::ast::Gender;
            match g.value {
                Gender::Male => "male",
                Gender::Female => "female",
                Gender::Other => "other",
            }
            .to_string()
        }),
        PersonField::Born => person.born().map(|d| d.format_canonical()),
        PersonField::Died => person.died().map(|d| d.format_canonical()),
    }
}

/// Parse one `--where` expression into predicates (a single comparison, or —
/// for a `!=` set membership — a conjunction of `Neq` predicates). The CLI
/// owns the *grammar*; the core owns the *semantics* (date parsing, three-
/// valued evaluation), so an invalid date literal or an ordering op on a
/// non-date field surfaces later as a core diagnostic, not here.
///
/// # Errors
///
/// A message when the expression names an unknown field, uses an unknown
/// operator, has an empty value, or mixes `|` with an ordering operator.
pub fn parse_where(expr: &str) -> Result<Vec<Predicate>, String> {
    let expr = expr.trim();
    if let Some(field) = paren_form(expr, "present") {
        return Ok(vec![Predicate::Present {
            field: parse_field(field)?,
        }]);
    }
    if let Some(field) = paren_form(expr, "absent") {
        return Ok(vec![Predicate::Absent {
            field: parse_field(field)?,
        }]);
    }

    // The field is a run of ASCII letters; the first non-letter starts the op.
    let boundary = expr
        .find(|c: char| !c.is_ascii_alphabetic())
        .ok_or_else(|| {
            format!(
                "cannot parse filter `{expr}`: expected an operator \
             (=, !=, <, <=, >, >=) or present(FIELD) / absent(FIELD)"
            )
        })?;
    let (field_str, rest) = expr.split_at(boundary);
    let field = parse_field(field_str)?;

    // Two-character operators before their one-character prefixes.
    let (kind, value) = if let Some(v) = rest.strip_prefix("!=") {
        (OpKind::Neq, v)
    } else if let Some(v) = rest.strip_prefix("<=") {
        (OpKind::Lte, v)
    } else if let Some(v) = rest.strip_prefix(">=") {
        (OpKind::Gte, v)
    } else if let Some(v) = rest.strip_prefix('<') {
        (OpKind::Lt, v)
    } else if let Some(v) = rest.strip_prefix('>') {
        (OpKind::Gt, v)
    } else if let Some(v) = rest.strip_prefix('=') {
        (OpKind::Eq, v)
    } else {
        return Err(format!(
            "cannot parse filter `{expr}`: unknown operator (use =, !=, <, <=, >, >=)"
        ));
    };
    if value.is_empty() {
        return Err(format!("filter `{expr}` has an empty value"));
    }

    let has_pipe = value.contains('|');
    match kind {
        OpKind::Eq if has_pipe => Ok(vec![Predicate::In {
            field,
            values: split_members(value, expr)?,
        }]),
        OpKind::Neq if has_pipe => Ok(split_members(value, expr)?
            .into_iter()
            .map(|value| Predicate::Neq { field, value })
            .collect()),
        _ if has_pipe => Err(format!(
            "filter `{expr}`: `|` set membership is only valid with `=` (∈) or `!=` (∉)"
        )),
        OpKind::Eq => Ok(vec![Predicate::Eq {
            field,
            value: value.to_string(),
        }]),
        OpKind::Neq => Ok(vec![Predicate::Neq {
            field,
            value: value.to_string(),
        }]),
        OpKind::Lt => Ok(vec![Predicate::Lt {
            field,
            value: value.to_string(),
        }]),
        OpKind::Lte => Ok(vec![Predicate::Lte {
            field,
            value: value.to_string(),
        }]),
        OpKind::Gt => Ok(vec![Predicate::Gt {
            field,
            value: value.to_string(),
        }]),
        OpKind::Gte => Ok(vec![Predicate::Gte {
            field,
            value: value.to_string(),
        }]),
    }
}

/// The comparison operators the `--where` grammar recognizes.
enum OpKind {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

/// Split a pipe-separated set into non-empty members, erroring on an empty
/// segment (`family=A|`).
fn split_members(value: &str, expr: &str) -> Result<Vec<String>, String> {
    let members: Vec<String> = value.split('|').map(str::to_string).collect();
    if members.iter().any(|m| m.is_empty()) {
        return Err(format!("filter `{expr}` has an empty value in its `|` set"));
    }
    Ok(members)
}

/// Match a `name(inner)` presence form, returning the trimmed inner text.
fn paren_form<'a>(expr: &'a str, name: &str) -> Option<&'a str> {
    expr.strip_prefix(name)
        .and_then(|r| r.strip_prefix('('))
        .and_then(|r| r.strip_suffix(')'))
        .map(str::trim)
}

/// Parse a `--sort` spec: `FIELD[:asc|:desc]` (asc default).
///
/// # Errors
///
/// A message when the field is unknown or the direction is not `asc`/`desc`.
pub fn parse_sort(spec: &str) -> Result<SortSpec, String> {
    let (field_str, direction) = match spec.split_once(':') {
        Some((field, "asc")) => (field, SortDirection::Asc),
        Some((field, "desc")) => (field, SortDirection::Desc),
        Some((_, other)) => {
            return Err(format!(
                "cannot parse sort `{spec}`: direction must be `asc` or `desc`, got `{other}`"
            ));
        }
        None => (spec, SortDirection::Asc),
    };
    Ok(SortSpec {
        field: parse_field(field_str)?,
        direction,
    })
}

/// Parse a person field name from the `--where` / `--sort` grammar.
fn parse_field(name: &str) -> Result<PersonField, String> {
    Ok(match name {
        "id" => PersonField::Id,
        "name" => PersonField::Name,
        "family" => PersonField::Family,
        "given" => PersonField::Given,
        "gender" => PersonField::Gender,
        "born" => PersonField::Born,
        "died" => PersonField::Died,
        other => {
            return Err(format!(
                "unknown field `{other}` \
                 (expected id, name, family, given, gender, born, or died)"
            ));
        }
    })
}

// ---- Relationship resolution (`kul query rel <x> <y>`) ----

/// Options for a `kul query rel` run: the two anchor ids, the resolution
/// config (the generation budget), and the output format.
pub struct RelOptions {
    pub x: String,
    pub y: String,
    pub config: ResolveConfig,
    pub format: OutputFormat,
}

pub fn run_rel(opts: RelOptions) -> ExitCode {
    let (_project, check) = match load_and_check() {
        Ok(x) => x,
        Err(code) => return code,
    };
    // One evaluation path: the same envelope the WASM `queryResolve` surface
    // returns, so `--format json` bytes are byte-identical.
    let envelope = resolve_relationship(&check, &opts.x, &opts.y, &opts.config);
    match opts.format {
        OutputFormat::Json => finish_rel_json(&envelope),
        OutputFormat::Human => finish_rel_human(&envelope, &opts, &check),
    }
}

fn finish_rel_json(envelope: &QueryEnvelope<ResolveResult>) -> ExitCode {
    // The envelope IS the contract answer for every outcome (an empty-with-
    // reason result, a bad id, a failing project) — always stdout, carrying the
    // diagnostic on the error arms. An empty result is an answer: exit 0.
    let json = serde_json::to_string(envelope).expect("serialize resolve envelope");
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = writeln!(out, "{json}") {
        eprintln!("kul: failed to write query envelope: {err}");
        return ExitCode::from(1);
    }
    match envelope {
        QueryEnvelope::Error(_) => ExitCode::from(1),
        QueryEnvelope::Ok(_) => ExitCode::SUCCESS,
    }
}

fn finish_rel_human(
    envelope: &QueryEnvelope<ResolveResult>,
    opts: &RelOptions,
    check: &CheckResult,
) -> ExitCode {
    match envelope {
        QueryEnvelope::Error(_) => {
            if check.has_errors() {
                // Load-and-check gate: render the project's diagnostics.
                diag::render_human(check, false);
            } else {
                // A clean project with an error arm can only be a bad id; the
                // synthesized diagnostic already names which one.
                for d in envelope_diagnostics(envelope) {
                    eprintln!("kul: {}", d);
                }
            }
            ExitCode::from(1)
        }
        QueryEnvelope::Ok(ok) => {
            print!("{}", render_resolve_human(&ok.result, opts, check));
            // An empty result is a complete answer — exit 0.
            ExitCode::SUCCESS
        }
    }
}

/// The messages of an error envelope's diagnostics (a bad-id resolve carries
/// exactly one, naming the offending id).
fn envelope_diagnostics(envelope: &QueryEnvelope<ResolveResult>) -> Vec<String> {
    match envelope {
        QueryEnvelope::Error(err) => err.diagnostics.iter().map(|d| d.message.clone()).collect(),
        QueryEnvelope::Ok(_) => Vec::new(),
    }
}

/// Render a resolution result for humans: one terminology-neutral block per
/// relationship (its descriptor facts plus the hop-by-hop path with ids and
/// display names), or the honest empty-reason wording when there is no tie.
fn render_resolve_human(result: &ResolveResult, opts: &RelOptions, check: &CheckResult) -> String {
    if result.relationships.is_empty() {
        return match result.empty_reason {
            Some(EmptyReason::Disconnected) => {
                "no connection: the two persons are in different family components\n".to_string()
            }
            // `None` cannot occur for an empty list (the core sets a reason iff
            // empty), but fall through to the bounds wording rather than panic.
            _ => format!(
                "no relationship found within {} generations (try --max-generations)\n",
                opts.config.max_apex_generations
            ),
        };
    }
    let mut out = String::new();
    for (i, descriptor) in result.relationships.iter().enumerate() {
        out.push_str(&format!(
            "relationship {}: {}\n",
            i + 1,
            descriptor_facts(descriptor)
        ));
        out.push_str(&render_path_human(descriptor, check));
    }
    out
}

/// The hop-by-hop path of a resolution descriptor, one hop per line, each with
/// the id landed on and its display name; `across` hops carry the marriage id,
/// status, and end reason. An empty path (the reflexive `self`) renders a
/// single explanatory line.
fn render_path_human(descriptor: &RelationshipDescriptor, check: &CheckResult) -> String {
    if descriptor.path.is_empty() {
        return "  (no path — same person)\n".to_string();
    }
    let name_of = |id: &str| {
        check
            .resolved()
            .person(id)
            .map(|p| p.display_name().to_string())
            .unwrap_or_else(|| id.to_string())
    };
    let mut out = String::new();
    for hop in &descriptor.path {
        match hop {
            PathHop::Up { to, edge, .. } => {
                out.push_str(&format!(
                    "  up {} {} ({})\n",
                    edge_word(*edge),
                    to,
                    name_of(to)
                ));
            }
            PathHop::Down { to, edge, .. } => {
                out.push_str(&format!(
                    "  down {} {} ({})\n",
                    edge_word(*edge),
                    to,
                    name_of(to)
                ));
            }
            PathHop::Across {
                to,
                marriage,
                status,
                end_reason,
                ..
            } => {
                out.push_str(&format!("  across {marriage} · {}", status_word(*status)));
                if let Some(reason) = end_reason {
                    out.push_str(&format!(" · {reason}"));
                }
                out.push_str(&format!(" {} ({})\n", to, name_of(to)));
            }
        }
    }
    out
}

/// The edge-kind token for a vertical hop.
fn edge_word(edge: HopEdge) -> &'static str {
    match edge {
        HopEdge::Bio => "bio",
        HopEdge::Adoptive => "adoptive",
    }
}
