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
//! `kula_lsp` surface (`features::diagnostics::to_lsp` + `convert::LineIndex`)
//! against `kula_core::check` end-to-end. No LSP-protocol round-trip — the
//! goal is to measure the language pipeline, not stdio framing.

use kula_lsp::convert::LineIndex;
use kula_lsp::features::diagnostics::to_lsp;
use tower_lsp::lsp_types::Url;

#[test]
fn one_thousand_statement_check_and_translate_under_budget() {
    let url = Url::parse("file:///t.kula").expect("valid url");

    let mut source = String::from("kula 1\n");
    for i in 0..1000 {
        use std::fmt::Write as _;
        let _ = writeln!(&mut source, "person p{i} name:\"P{i}\" gender:female");
    }

    let start = std::time::Instant::now();
    let core = kula_core::check(&source);
    let line_index = LineIndex::new(source.as_str());
    let _ = to_lsp(&url, &core.diagnostics, &line_index);
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
