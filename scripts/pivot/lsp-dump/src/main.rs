//! Behaviour dump of the `kul-lsp` feature functions over a directory of
//! example projects, for the epic #343 baseline (`scripts/pivot/baseline.sh`).
//!
//! Usage: `lsp-dump <examples-root> <out-dir>`
//!
//! Every subdirectory of `<examples-root>` that holds a `.kul` file is one
//! project: its `*.kul` files, read flat. The project is checked the way the
//! server checks it, then every feature function is called directly, with no
//! LSP session:
//!
//! - per file: diagnostics, document symbols, semantic tokens, formatting
//!   and code actions over the whole file;
//! - per position (every token start and end offset, deduplicated): hover,
//!   completion, definition, references (with and without the declaration),
//!   prepare-rename, rename to `pivot_renamed`, and entity-at;
//! - per project: locate for every person and marriage id.
//!
//! The pinned tree's `kul_core::check` also takes the project's manifest;
//! `baseline.sh` builds that side with the `pinned` feature, which reads it
//! the way the pinned server did (`kul_loader::discover`).
//!
//! Each project writes `<out-dir>/<project>.json`. Object keys are sorted
//! (serde_json without `preserve_order`), URLs are `file:///<project>/<file>`,
//! so the output is byte-stable across machines and checkouts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kul_core::ast::InputFile;
use kul_core::lexer::tokenize;
use kul_core::span::FileId;
use kul_lsp::convert::LineIndex;
use kul_lsp::features::{
    code_action, completion, definition, diagnostics, document_symbol, entity_at, formatting,
    hover, locate, references, rename, semantic_tokens,
};
use kul_lsp::state::{ProjectEntry, ProjectRoot};
use serde_json::{Value, json};
use tower_lsp::lsp_types::{Position, Range, Url};

const RENAME_TO: &str = "pivot_renamed";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: lsp-dump <examples-root> <out-dir>");
        std::process::exit(2);
    }
    let root = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out).expect("create out dir");

    let mut projects: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("read examples root")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir() && has_kul_file(p))
        .collect();
    projects.sort();

    for dir in &projects {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let dump = dump_project(&name, dir);
        let text = serde_json::to_string_pretty(&dump).expect("serialize dump");
        std::fs::write(out.join(format!("{name}.json")), text + "\n").expect("write dump");
        eprintln!("lsp-dump: {name}");
    }
    let legend = serde_json::to_value(semantic_tokens::legend()).expect("legend");
    let text = serde_json::to_string_pretty(&legend).expect("serialize legend");
    std::fs::write(out.join("_legend.json"), text + "\n").expect("write legend");
}

fn has_kul_file(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .any(|e| e.path().extension().and_then(|s| s.to_str()) == Some("kul"))
        })
        .unwrap_or(false)
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

#[cfg(feature = "pinned")]
fn check(dir: &Path, inputs: &[InputFile]) -> kul_core::CheckResult {
    let discovered = kul_loader::discover(dir);
    kul_core::check(discovered.manifest_name, &discovered.manifest_yaml, inputs)
}

#[cfg(not(feature = "pinned"))]
fn check(_dir: &Path, inputs: &[InputFile]) -> kul_core::CheckResult {
    kul_core::check(inputs)
}

fn build_entry(project: &str, dir: &Path) -> ProjectEntry {
    let mut files: Vec<(Url, Arc<str>, String)> = read_kul_files(dir)
        .into_iter()
        .map(|(name, source)| {
            let url = Url::parse(&format!("file:///{project}/{name}")).expect("file url");
            (url, Arc::from(source), name)
        })
        .collect();
    // Both trees' servers and checks agree on this order for the examples'
    // plain ASCII names.
    files.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
    let inputs: Vec<InputFile> = files
        .iter()
        .map(|(_, src, name)| InputFile::new(name.clone(), Arc::clone(src)))
        .collect();
    let check = check(dir, &inputs);
    let metas = files
        .iter()
        .map(|(url, src, _)| (url.clone(), LineIndex::new(Arc::clone(src))))
        .collect();
    ProjectEntry::from_parts(
        ProjectRoot::from_path(PathBuf::from(format!("/{project}"))),
        check,
        metas,
        Default::default(),
    )
}

fn dump_project(project: &str, dir: &Path) -> Value {
    let entry = build_entry(project, dir);
    let urls: Vec<Url> = entry.project_urls().cloned().collect();

    let mut files = BTreeMap::new();
    for url in &urls {
        let file = entry.file_id_for(url).expect("file id");
        let name = url.path().rsplit('/').next().unwrap_or_default().to_owned();
        files.insert(name, dump_file(&entry, url, file));
    }

    let resolved = entry.check.resolved();
    let mut ids: BTreeSet<String> = resolved.persons().map(|p| p.id.name.clone()).collect();
    ids.extend(resolved.marriages().map(|m| m.id.name.clone()));
    ids.insert("no_such_entity".to_owned());
    let any_url = urls.first().cloned();
    let locate: BTreeMap<String, Value> = match any_url {
        Some(uri) => ids
            .into_iter()
            .map(|id| {
                let params = locate::LocateParams {
                    uri: uri.clone(),
                    id: id.clone(),
                };
                (id, to_value(locate::locate(&entry, &params)))
            })
            .collect(),
        None => BTreeMap::new(),
    };

    json!({
        "project": project,
        "files": files,
        "locate": locate,
    })
}

fn dump_file(entry: &ProjectEntry, url: &Url, file: FileId) -> Value {
    let line_index = entry.line_index_for(file).expect("line index");
    let source = line_index.source().to_owned();
    let check = &entry.check;
    let resolved = check.resolved();

    let whole_file = Range {
        start: Position::new(0, 0),
        end: line_index.position(source.len()),
    };
    let code_actions = code_action::code_actions(
        file,
        resolved,
        &check.diagnostics,
        line_index,
        url,
        whole_file,
    );

    let mut offsets: BTreeSet<usize> = BTreeSet::new();
    for token in tokenize(&source) {
        offsets.insert(token.span.start);
        offsets.insert(token.span.end);
    }

    let positions: Vec<Value> = offsets
        .into_iter()
        .map(|offset| dump_position(entry, url, &source, line_index, offset))
        .collect();

    json!({
        "diagnostics": to_value(diagnostics::to_lsp(entry, file, &check.diagnostics)),
        "symbols": to_value(document_symbol::document_symbols(file, resolved, line_index)),
        "semanticTokens": to_value(semantic_tokens::semantic_tokens(file, resolved, line_index)),
        "formatting": to_value(formatting::formatting(&source, &check.diagnostics, line_index, file)),
        "codeActions": to_value(code_actions),
        "positions": positions,
    })
}

fn dump_position(
    entry: &ProjectEntry,
    url: &Url,
    source: &str,
    line_index: &LineIndex,
    offset: usize,
) -> Value {
    let position = line_index.position(offset);
    let cursor = entry.cursor_for_uri(url, position);
    let hover = cursor
        .as_ref()
        .and_then(|c| hover::hover(c.file, c.resolved, c.line_index, c.offset));
    let completion = cursor
        .as_ref()
        .map(|c| completion::complete(source, c.file, c.resolved, c.offset));
    let prepare_rename = cursor
        .as_ref()
        .and_then(|c| rename::prepare_rename(c.file, c.resolved, c.line_index, c.offset));
    let rename = match rename::rename(entry, url, position, RENAME_TO) {
        Ok(edit) => json!({ "ok": to_value(edit) }),
        Err(e) => json!({ "error": e.message() }),
    };
    let entity = entity_at::entity_at(
        entry,
        &entity_at::EntityAtParams {
            uri: url.clone(),
            position,
        },
    );

    json!({
        "offset": offset,
        "position": to_value(position),
        "hover": to_value(hover),
        "completion": to_value(completion),
        "definition": to_value(definition::definition(entry, url, position)),
        "references": to_value(references::references(entry, url, position, false)),
        "referencesWithDeclaration": to_value(references::references(entry, url, position, true)),
        "prepareRename": to_value(prepare_rename),
        "rename": rename,
        "entityAt": to_value(entity),
    })
}
