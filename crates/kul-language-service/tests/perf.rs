//! Performance budget gates for the per-edit language pipeline: the check
//! plus the diagnostics projection every analysis pays. Each test asserts a
//! generous (~5×) ceiling around the real target, so 2× regressions still
//! fire while CI runner variance is absorbed.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use kul_core::ast::InputFile;

#[test]
fn one_thousand_statement_check_and_diagnostics_under_budget() {
    let mut source = String::new();
    for i in 0..1000 {
        let _ = writeln!(&mut source, "person p{i} name:\"P{i}\" gender:female");
    }

    let start = Instant::now();
    let check = kul_core::check(&[InputFile::new("test.kul", source.as_str())]);
    let diagnostics = kul_language_service::diagnostics(&check);
    let elapsed = start.elapsed();

    assert!(diagnostics.is_empty());
    eprintln!("1000-statement check + diagnostics: {elapsed:?}");
    // Target 100ms; the 500ms ceiling absorbs CI and debug-build variance.
    assert!(
        elapsed < Duration::from_millis(500),
        "1000-statement budget exceeded: {elapsed:?}"
    );
}

#[test]
fn ten_files_of_one_hundred_statements_under_budget() {
    // Per-project budget (ADR-0015). Regressing faster than the single-file
    // 1000-statement test above signals a per-file cost in the resolver.
    let inputs: Vec<InputFile> = (0..10)
        .map(|f| {
            let mut source = String::new();
            for i in 0..100 {
                let _ = writeln!(
                    &mut source,
                    "person p_f{f}_i{i} name:\"P{f}.{i}\" gender:female"
                );
            }
            InputFile::new(format!("file{f}.kul"), source)
        })
        .collect();

    let start = Instant::now();
    let check = kul_core::check(&inputs);
    let diagnostics = kul_language_service::diagnostics(&check);
    let elapsed = start.elapsed();

    assert!(diagnostics.is_empty());
    eprintln!("10x100-statement multi-file check + diagnostics: {elapsed:?}");
    // Target 100ms; 500ms ceiling, as above.
    assert!(
        elapsed < Duration::from_millis(500),
        "10x100 multi-file budget exceeded: {elapsed:?}"
    );
}
