//! Property tests for the formatter against the example corpus and the
//! valid-input slice of the validator corpus.
//!
//! Two properties land here, both per [ADR 0004]:
//!
//! - **Idempotence**: `format_source(format_source(s)) == format_source(s)`.
//!   Once the formatter has touched a file, running it again must be a
//!   no-op. Tested byte-for-byte.
//! - **Round-trip**: `parse(format_source(s))` produces an AST equivalent
//!   to `parse(s)` modulo span positions. We use `format(&Document)` as a
//!   span-erasing canonical form: two ASTs that print to the same string
//!   under the AST-only formatter are structurally equal.
//!
//! [ADR 0004]: https://github.com/YashBhalodi/kulalang/blob/main/docs/adr/0004-formatter-canonical-rules.md

use std::path::{Path, PathBuf};

use kula_core::format::{format, format_source};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/kula-core; the workspace root is
    // two levels up.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn corpus_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let examples = workspace_root().join("examples");
    if examples.is_dir() {
        for entry in std::fs::read_dir(&examples)
            .expect("read examples dir")
            .flatten()
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("kula") {
                files.push(path);
            }
        }
    }
    let valid = workspace_root().join("crates/kula-core/tests/corpus/valid");
    if valid.is_dir() {
        for entry in std::fs::read_dir(&valid)
            .expect("read valid corpus")
            .flatten()
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("kula") {
                files.push(path);
            }
        }
    }
    files.sort();
    assert!(
        !files.is_empty(),
        "no corpus files found under {examples:?} or valid corpus"
    );
    files
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

#[test]
fn format_source_is_idempotent_on_corpus() {
    for path in corpus_files() {
        let source = read(&path);
        let once = format_source(&source);
        let twice = format_source(&once);
        assert_eq!(
            once,
            twice,
            "format_source not idempotent for {}\n--- once ---\n{once}\n--- twice ---\n{twice}",
            path.display()
        );
    }
}

#[test]
fn format_source_round_trips_ast_through_corpus() {
    for path in corpus_files() {
        let source = read(&path);
        let original_ast = kula_core::check(&source).document;
        let formatted = format_source(&source);
        let reparsed_ast = kula_core::check(&formatted).document;
        // `format(&Document)` is span-blind, so two ASTs that print equal
        // are equal modulo span positions — exactly the equivalence we want.
        assert_eq!(
            format(&original_ast),
            format(&reparsed_ast),
            "AST round-trip failed for {}",
            path.display()
        );
    }
}

#[test]
fn format_ast_only_is_idempotent_on_corpus() {
    for path in corpus_files() {
        let source = read(&path);
        let doc1 = kula_core::check(&source).document;
        let printed_once = format(&doc1);
        let doc2 = kula_core::check(&printed_once).document;
        let printed_twice = format(&doc2);
        assert_eq!(
            printed_once,
            printed_twice,
            "AST-only format not idempotent for {}",
            path.display()
        );
    }
}
