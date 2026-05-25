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
//! [ADR 0004]: https://github.com/YashBhalodi/kul/blob/main/docs/adr/0004-formatter-canonical-rules.md

mod common;

use std::path::{Path, PathBuf};

use kul_core::ast::KulFile;
use kul_core::format::{format, format_source};

use crate::common::check_one;

fn first_kul_file(check: &kul_core::CheckResult) -> KulFile {
    let doc = check.document();
    let arc = doc
        .kul_files
        .first()
        .expect("at least one .kul file")
        .clone();
    KulFile::clone(&arc)
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/kul-core; the workspace root is
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
        // Each per-example subdirectory carries one or more `.kul` files
        // alongside its `kul.yml`. Walk one level deep into every subdir.
        for example_dir in std::fs::read_dir(&examples)
            .expect("read examples dir")
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
        {
            for entry in std::fs::read_dir(&example_dir)
                .expect("read example subdirectory")
                .flatten()
            {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("kul") {
                    files.push(path);
                }
            }
        }
    }
    let valid = workspace_root().join("crates/kul-core/tests/corpus/valid");
    if valid.is_dir() {
        // Like `examples/`, each fixture is a one-file project in its own
        // subdirectory (`<name>/<name>.kul` beside a `kul.yml`); descend one
        // level into every subdir to collect the `.kul` files.
        for fixture_dir in std::fs::read_dir(&valid)
            .expect("read valid corpus")
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
        {
            for entry in std::fs::read_dir(&fixture_dir)
                .expect("read fixture subdirectory")
                .flatten()
            {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("kul") {
                    files.push(path);
                }
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
        let original_ast = first_kul_file(&check_one(&source));
        let formatted = format_source(&source);
        let reparsed_ast = first_kul_file(&check_one(&formatted));
        // `format(&KulFile)` is span-blind, so two ASTs that print equal
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
        let kf1 = first_kul_file(&check_one(&source));
        let printed_once = format(&kf1);
        let kf2 = first_kul_file(&check_one(&printed_once));
        let printed_twice = format(&kf2);
        assert_eq!(
            printed_once,
            printed_twice,
            "AST-only format not idempotent for {}",
            path.display()
        );
    }
}
