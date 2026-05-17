//! Performance budget tests.
//!
//! These tests are budget gates, not benchmarks: each one asserts an upper
//! bound on wall-clock time for a representative workload. They run as part
//! of the regular test suite (no separate `cargo bench` step) so a
//! regression fires immediately on every PR.
//!
//! Budget choices document the *real* target in a comment and assert a
//! generous ceiling (typically 5×) to absorb CI runner variability without
//! masking 2× regressions. If a legitimate change needs more headroom,
//! raise the ceiling deliberately and update the comment.
//!
//! Tests live here rather than inline because they exercise the full public
//! `kul_lsp` surface (`features::diagnostics::to_lsp` + `convert::LineIndex`)
//! against `kul_core::check` end-to-end. No LSP-protocol round-trip — the
//! goal is to measure the language pipeline, not stdio framing.

use std::collections::HashMap;
use std::path::PathBuf;

use kul_lsp::convert::LineIndex;
use kul_lsp::features::diagnostics::to_lsp;
use kul_lsp::state::{ProjectEntry, ProjectRoot};
use tower_lsp::lsp_types::Url;

/// Hand-build a `ProjectEntry` from already-checked inputs. Perf-test
/// equivalent of `did_open` without going through `Documents` (no
/// disk-reading, no overlay management, no async). Mirrors the URL
/// shape `Documents::open` produces for `file:///<basename>` URIs.
fn fixture_entry(check: kul_core::CheckResult, basenames: &[&str]) -> ProjectEntry {
    let urls: Vec<Url> = basenames
        .iter()
        .map(|name| Url::parse(&format!("file:///{name}")).expect("valid url"))
        .collect();
    let line_indices: Vec<LineIndex> = check
        .document()
        .kul_file_ids()
        .map(|f| LineIndex::new(check.document().source_of(f).unwrap()))
        .collect();
    ProjectEntry {
        root: ProjectRoot::from_path(PathBuf::from("/perf")),
        check,
        line_indices,
        urls,
        overlay: HashMap::new(),
    }
}

#[test]
fn one_thousand_statement_check_and_translate_under_budget() {
    let mut source = String::new();
    for i in 0..1000 {
        use std::fmt::Write as _;
        let _ = writeln!(&mut source, "person p{i} name:\"P{i}\" gender:female");
    }

    let start = std::time::Instant::now();
    let inputs = vec![kul_core::ast::InputFile::new("test.kul", source.as_str())];
    let core = kul_core::check_with_manifest(
        "kul.yml",
        "",
        &kul_core::manifest::Manifest::default(),
        &inputs,
    );
    let file = core.document().kul_file_ids().next().unwrap();
    let diagnostics = core.diagnostics.clone();
    let entry = fixture_entry(core, &["test.kul"]);
    let _ = to_lsp(&entry, file, &diagnostics);
    let elapsed = start.elapsed();

    eprintln!("1000-statement parse + check + to_lsp: {elapsed:?}");
    // PRD target is 100ms. CI runners and debug builds are slower than a
    // developer laptop, so assert a generous 500ms ceiling — enough to
    // catch a 5x regression without flaking.
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "1000-statement budget exceeded: {elapsed:?}"
    );
}

#[test]
fn ten_files_of_one_hundred_statements_under_budget() {
    // Multi-file shape per [ADR-0015](../../docs/adr/0015-global-project-namespace.md):
    // ten 100-statement `.kul` files in one project = 1000 total
    // statements. Under project-wide resolution the resolver walks
    // every file in one pass; the budget is the same 100ms target
    // (asserted at 500ms for CI slack) re-interpreted *per project*
    // rather than per file. If this test regresses faster than the
    // single-file 1000-statement one above, the file-count loop in the
    // resolver has acquired a cost it shouldn't have.
    let mut inputs: Vec<kul_core::ast::InputFile> = Vec::with_capacity(10);
    let mut basenames: Vec<String> = Vec::with_capacity(10);
    for f in 0..10 {
        let mut source = String::new();
        for i in 0..100 {
            use std::fmt::Write as _;
            let _ = writeln!(
                &mut source,
                "person p_f{f}_i{i} name:\"P{f}.{i}\" gender:female"
            );
        }
        let name = format!("file{f}.kul");
        inputs.push(kul_core::ast::InputFile::new(name.clone(), source));
        basenames.push(name);
    }

    let start = std::time::Instant::now();
    let core = kul_core::check_with_manifest(
        "kul.yml",
        "",
        &kul_core::manifest::Manifest::default(),
        &inputs,
    );
    let diagnostics = core.diagnostics.clone();
    let file_ids: Vec<_> = core.document().kul_file_ids().collect();
    let basename_refs: Vec<&str> = basenames.iter().map(String::as_str).collect();
    let entry = fixture_entry(core, &basename_refs);
    // Translate per-file diagnostics for every file — matches what the
    // LSP slice will do when it broadcasts diagnostics for every URI
    // in a project (per the PRD).
    for file in file_ids {
        let _ = to_lsp(&entry, file, &diagnostics);
    }
    let elapsed = start.elapsed();

    eprintln!("10x100-statement multi-file parse + check + to_lsp: {elapsed:?}");
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "10x100 multi-file budget exceeded: {elapsed:?}"
    );
}
