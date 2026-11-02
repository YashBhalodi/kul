//! Behaviour dump of `kul-language-service` over a directory of example
//! projects, for the epic #343 baseline (`scripts/pivot/baseline.sh`). The
//! current-tree counterpart of `lsp-dump`, which captures the pinned tree's
//! language server; `compare.mjs` maps that capture onto this one's shapes.
//!
//! Usage: `service-dump <examples-root> <out-dir>`
//!
//! Every subdirectory of `<examples-root>` that holds a `.kul` file is one
//! project: its `*.kul` files, read flat, checked once. Then every feature is
//! called with the same inputs lsp-dump uses:
//!
//! - per file: diagnostics, symbols, semantic tokens, format and quick fixes
//!   over the whole file;
//! - per position (every token start and end offset, deduplicated): hover,
//!   completion, definition, references (with and without the declaration),
//!   prepare-rename, rename to `pivot_renamed`, and entity-at;
//! - per project: locate for every person and marriage id, plus one unknown id.
//!
//! Each project writes `<out-dir>/<project>.json` with sorted object keys, so
//! the output is byte-stable across machines and checkouts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::lexer::tokenize;
use kul_language_service as service;
use serde_json::{Value, json};

const RENAME_TO: &str = "pivot_renamed";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: service-dump <examples-root> <out-dir>");
        std::process::exit(2);
    }
    let root = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out).expect("create out dir");

    let mut projects: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("read examples root")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir() && !read_kul_files(p).is_empty())
        .collect();
    projects.sort();

    for dir in &projects {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let dump = dump_project(&name, dir);
        let text = serde_json::to_string_pretty(&dump).expect("serialize dump");
        std::fs::write(out.join(format!("{name}.json")), text + "\n").expect("write dump");
        eprintln!("service-dump: {name}");
    }
}

fn to_value<T: serde::Serialize>(v: T) -> Value {
    serde_json::to_value(v).expect("serialize feature result")
}

/// `(name, source)` of every `*.kul` file directly inside `dir`.
fn read_kul_files(dir: &Path) -> Vec<(String, String)> {
    std::fs::read_dir(dir)
        .expect("read project dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul") && !p.is_dir())
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let source = std::fs::read_to_string(&p).expect("read .kul file");
            (name, source)
        })
        .collect()
}

fn dump_project(project: &str, dir: &Path) -> Value {
    let inputs: Vec<InputFile> = read_kul_files(dir)
        .into_iter()
        .map(|(name, source)| InputFile::new(name, source))
        .collect();
    let check = kul_core::check(&inputs);

    let mut files = BTreeMap::new();
    for (_, file) in check.document().kul_files() {
        files.insert(
            file.name.clone(),
            dump_file(&check, &file.name, &file.source),
        );
    }

    let resolved = check.resolved();
    let mut ids: BTreeSet<String> = resolved.persons().map(|p| p.id.name.clone()).collect();
    ids.extend(resolved.marriages().map(|m| m.id.name.clone()));
    ids.insert("no_such_entity".to_owned());
    let locate: BTreeMap<String, Value> = ids
        .into_iter()
        .map(|id| {
            let location = to_value(service::locate(&check, &id));
            (id, location)
        })
        .collect();

    json!({
        "project": project,
        "files": files,
        "locate": locate,
    })
}

fn dump_file(check: &CheckResult, file: &str, source: &str) -> Value {
    let diagnostics: Vec<service::Diagnostic> = service::diagnostics(check)
        .into_iter()
        .filter(|d| d.location.as_ref().is_some_and(|l| l.file == file))
        .collect();
    let whole_file = service::Span {
        start: 0,
        end: source.len(),
    };

    let mut offsets: BTreeSet<usize> = BTreeSet::new();
    for token in tokenize(source) {
        offsets.insert(token.span.start);
        offsets.insert(token.span.end);
    }
    let positions: Vec<Value> = offsets
        .into_iter()
        .map(|offset| dump_position(check, file, offset))
        .collect();

    json!({
        "diagnostics": to_value(diagnostics),
        "symbols": to_value(service::symbols(check, file)),
        "semanticTokens": to_value(service::semantic_tokens(check, file)),
        "format": to_value(service::format(check, file)),
        "quickFixes": to_value(service::quick_fixes(check, file, whole_file)),
        "positions": positions,
    })
}

fn dump_position(check: &CheckResult, file: &str, offset: usize) -> Value {
    let rename = match service::rename(check, file, offset, RENAME_TO) {
        Ok(edits) => json!({ "ok": to_value(edits) }),
        Err(e) => json!({ "error": e.message() }),
    };
    json!({
        "offset": offset,
        "hover": to_value(service::hover(check, file, offset)),
        "completion": to_value(service::complete(check, file, offset)),
        "definition": to_value(service::definition(check, file, offset)),
        "references": to_value(service::references(check, file, offset, false)),
        "referencesWithDeclaration": to_value(service::references(check, file, offset, true)),
        "prepareRename": to_value(service::prepare_rename(check, file, offset)),
        "rename": rename,
        "entityAt": to_value(service::entity_at(check, file, offset)),
    })
}
