//! Performance budget gate for one `Workspace::analyze()`, the work the
//! browser does after each burst of typing: check the project, project its
//! diagnostics to UTF-16 offsets and render the tree. Measured natively; the
//! browser measurement lives with the canvas's end-to-end suite.

mod common;

use std::time::{Duration, Instant};

use common::example_files;
use kul_wasm::Workspace;

/// Median of `runs` timings of `f`.
fn median(runs: usize, mut f: impl FnMut() -> Duration) -> Duration {
    let mut times: Vec<Duration> = (0..runs).map(|_| f()).collect();
    times.sort();
    times[runs / 2]
}

#[test]
fn analyze_example_09_under_budget() {
    let files = example_files("09-family-across-a-century");
    let (first, _) = &files[0];

    let elapsed = median(9, || {
        let mut ws = Workspace::new();
        for (name, text) in &files {
            ws.set_file(name.clone(), text.clone(), 1);
        }
        // An edit, so the analysis re-checks the way it does after typing.
        let (_, text) = &files[0];
        ws.set_file(first.clone(), format!("{text}# edited\n"), 2);
        let start = Instant::now();
        let analysis = ws.analyze();
        let elapsed = start.elapsed();
        assert!(analysis.diagnostics.is_empty());
        assert!(analysis.svg.is_some());
        elapsed
    });

    eprintln!("analyze() on 09-family-across-a-century (median of 9): {elapsed:?}");
    // Targets, recorded in docs/testing.md: 2ms in a release build, 10ms in
    // a debug build; the ceilings are 5× those. Observed on a developer
    // laptop: ≈0.2ms release, ≈1.2ms debug.
    let ceiling = if cfg!(debug_assertions) {
        Duration::from_millis(50)
    } else {
        Duration::from_millis(10)
    };
    assert!(
        elapsed < ceiling,
        "analyze() budget exceeded: {elapsed:?} >= {ceiling:?}"
    );
}
