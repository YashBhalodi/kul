//! The diagnostic catalogue (`kul_core::diagnostic::DIAGNOSTIC_CODES`) lists
//! exactly the codes the workspace emits.
//!
//! Codes are string literals at the place each problem is found, across
//! kul-core, kul-render and the query engine. This test reads every Rust
//! source under `crates/*/src` and compares the `"KUL-…"` literals it finds
//! with the catalogue, both ways. The docs' diagnostics reference is checked
//! against the catalogue, so a new code needs an entry in both.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kul_core::diagnostic::DIAGNOSTIC_CODES;

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("kul-core sits inside crates/")
        .to_path_buf()
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// `"KUL-X00"` literals (with an optional lower-case suffix) in `text`.
fn code_literals(text: &str) -> Vec<String> {
    let mut codes = Vec::new();
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find("\"KUL-") {
        let start = at + from + 1;
        let mut end = start + "KUL-".len();
        let is_code_letter = bytes.get(end).is_some_and(u8::is_ascii_uppercase);
        if is_code_letter {
            end += 1;
            let digits_start = end;
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            while bytes.get(end).is_some_and(u8::is_ascii_lowercase) {
                end += 1;
            }
            if end > digits_start && bytes.get(end) == Some(&b'"') {
                codes.push(text[start..end].to_owned());
            }
        }
        from = start;
    }
    codes
}

fn emitted_codes() -> BTreeSet<String> {
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(crates_dir()).expect("read crates/") {
        let src = entry.expect("crate entry").path().join("src");
        if src.is_dir() {
            rust_sources(&src, &mut sources);
        }
    }
    assert!(
        !sources.is_empty(),
        "no Rust sources found under crates/*/src"
    );
    sources
        .iter()
        .filter(|path| !path.ends_with("kul-core/src/diagnostic.rs"))
        .flat_map(|path| {
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
            code_literals(&text)
        })
        .collect()
}

#[test]
fn the_catalogue_lists_every_code_the_sources_emit() {
    let catalogue: BTreeSet<String> = DIAGNOSTIC_CODES
        .iter()
        .map(|entry| entry.code.to_owned())
        .collect();
    let emitted = emitted_codes();
    let missing: Vec<_> = emitted.difference(&catalogue).collect();
    let unused: Vec<_> = catalogue.difference(&emitted).collect();
    assert!(
        missing.is_empty(),
        "codes emitted but not in DIAGNOSTIC_CODES: {missing:?}"
    );
    assert!(
        unused.is_empty(),
        "codes in DIAGNOSTIC_CODES that no source emits: {unused:?}"
    );
}

#[test]
fn the_catalogue_is_in_code_order_without_repeats() {
    let codes: Vec<&str> = DIAGNOSTIC_CODES.iter().map(|entry| entry.code).collect();
    let layer = |code: &str| match code.as_bytes()[4] {
        b'L' => 0,
        b'P' => 1,
        b'R' => 2,
        b'V' => 3,
        b'Q' => 4,
        other => panic!("unknown layer letter {}", other as char),
    };
    let mut sorted = codes.clone();
    sorted.sort_by_key(|code| (layer(code), *code));
    sorted.dedup();
    assert_eq!(codes, sorted);
    assert!(
        DIAGNOSTIC_CODES
            .iter()
            .all(|entry| !entry.summary.is_empty())
    );
}

#[test]
fn literals_are_found_with_and_without_a_suffix() {
    let text = r#"x("KUL-R05", y); z == "KUL-R05b"; starts_with("KUL-L"); "KUL-p01"; "KUL-P01 x""#;
    assert_eq!(code_literals(text), ["KUL-R05", "KUL-R05b"]);
}
