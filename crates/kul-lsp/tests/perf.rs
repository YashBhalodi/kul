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
use std::fmt::Write as _;
use std::path::PathBuf;

use kul_layout::{LayoutConfig, layout};
use kul_lsp::convert::LineIndex;
use kul_lsp::features::diagnostics::to_lsp;
use kul_lsp::state::{ProjectEntry, ProjectRoot};
use kul_render::{RenderShape, compute};
use kul_svg::{ThemeConfig, render};
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

// ---------------------------------------------------------------------------
// Large-document render gate (F14)
// ---------------------------------------------------------------------------

/// Append a person line plus an optional `birth` sub-statement to `out`.
fn emit_person(out: &mut String, pid: usize, gender: &str, born: u32, birth: Option<usize>) {
    let _ = writeln!(
        out,
        "person p{pid} name:\"Person {pid}\" gender:{gender} born:{born}"
    );
    if let Some(m) = birth {
        let _ = writeln!(out, "  birth m{m}");
    }
}

/// Grow a family subtree rooted at the already-declared person `spouse_a`:
/// marry in a fresh `spouse_b`, declare the marriage, then emit
/// `children_per` children whose `birth` references it. Every
/// `marry_every`-th child re-marries and recurses one generation deeper,
/// until `depth` is exhausted. All references point *backwards* (spouse
/// and marriage are declared before the children that cite them), so the
/// generated document needs no forward-resolution to validate.
///
/// Counters (`pc` next person id, `mc` next marriage id) thread through by
/// `&mut` so ids stay globally unique across every tree and singleton.
#[allow(clippy::too_many_arguments)]
fn grow_family(
    out: &mut String,
    pc: &mut usize,
    mc: &mut usize,
    spouse_a: usize,
    depth: usize,
    children_per: usize,
    marry_every: usize,
    born: u32,
) {
    let spouse_b = *pc;
    *pc += 1;
    // Alternate the married-in spouse's gender by id parity so the corpus
    // is not uniformly one-gendered; semantics don't constrain it.
    let gender = if spouse_b % 2 == 0 { "female" } else { "male" };
    emit_person(out, spouse_b, gender, born.saturating_sub(2), None);

    let m = *mc;
    *mc += 1;
    let _ = writeln!(out, "marriage m{m} p{spouse_a} p{spouse_b} start:{born}");

    for i in 0..children_per {
        let child = *pc;
        *pc += 1;
        let child_gender = if child % 2 == 0 { "female" } else { "male" };
        emit_person(out, child, child_gender, born + 25, Some(m));
        if depth > 1 && i % marry_every == 0 {
            grow_family(
                out,
                pc,
                mc,
                child,
                depth - 1,
                children_per,
                marry_every,
                born + 26,
            );
        }
    }
}

/// Generate a ~5,000-card Kul source string: a mix of shallow, medium, and
/// deep family trees (exercising Walker's recursion, marriage bars,
/// birth edges, generation rows, and multi-component packing) plus a tail
/// of stray singleton persons (no marriage, no birth — implicit
/// documentation roots). Built in-test rather than committed to
/// `examples/` so the corpus suite and its snapshots stay untouched.
///
/// Returns the source plus the count of declared persons (the card-count
/// upper bound — every person yields one canonical card).
fn generate_large_document(target_cards: usize) -> (String, usize) {
    let mut out = String::new();
    let mut pc = 0usize; // next person id
    let mut mc = 0usize; // next marriage id

    // Reserve ~10% of the budget for singletons; the rest goes to trees.
    let tree_budget = target_cards - target_cards / 10;

    // Cycle through tree shapes so depths and breadths vary across the
    // corpus rather than every tree being identical.
    let shapes = [
        // (depth, children_per, marry_every)
        (1, 3, 1), // shallow: a couple with a few childless children
        (2, 3, 2), // medium
        (3, 2, 1), // deep, narrow
        (4, 2, 2), // deep, mixed branching
    ];
    let mut shape_idx = 0usize;
    let mut year = 1900u32;

    while pc < tree_budget {
        let (depth, children_per, marry_every) = shapes[shape_idx % shapes.len()];
        shape_idx += 1;
        // Found the tree: a documentation-root person, then grow.
        let founder = pc;
        pc += 1;
        emit_person(&mut out, founder, "male", year, None);
        grow_family(
            &mut out,
            &mut pc,
            &mut mc,
            founder,
            depth,
            children_per,
            marry_every,
            year + 24,
        );
        // Drift the birth years so successive trees don't all overlap.
        year += 7;
        if year > 1980 {
            year = 1900;
        }
    }

    // Tail of stray singletons up to the target.
    while pc < target_cards {
        let s = pc;
        pc += 1;
        let gender = if s % 2 == 0 { "female" } else { "male" };
        emit_person(&mut out, s, gender, 1960, None);
    }

    (out, pc)
}

/// Count SVG element start-tags as a browser-DOM cost proxy: every tag
/// (open, self-close, or close) begins with `<`, so subtracting the
/// closing tags (`</`) leaves the number of elements emitted.
fn svg_node_count(svg: &str) -> usize {
    svg.matches('<').count() - svg.matches("</").count()
}

/// Large-document render gate: times the full `compute` → `layout` →
/// `render` chain — the exact pipeline `kul/render` runs on every edit —
/// over a ~5,000-card fixture, asserts a per-profile wall-clock ceiling
/// and a profile-independent SVG node-count ceiling, and logs the
/// per-stage breakdown so a future optimisation follow-up knows which
/// stage dominates without re-instrumenting.
///
/// This is measurement only — no optimisation. It pins a regression
/// ceiling and produces the per-stage data the maintainer uses to decide
/// whether large-document optimisation (virtualisation, culling,
/// incremental layout) is warranted at all (see issue #139 / F15).
#[test]
fn large_document_render_under_budget() {
    let (source, declared_persons) = generate_large_document(5_000);

    let inputs = vec![kul_core::ast::InputFile::new("large.kul", source.as_str())];
    let check = kul_core::check_with_manifest(
        "kul.yml",
        "",
        &kul_core::manifest::Manifest::default(),
        &inputs,
    );
    assert!(
        check.diagnostics.is_empty(),
        "generated fixture must validate cleanly; got {} diagnostics: {:?}",
        check.diagnostics.len(),
        check.diagnostics.iter().take(5).collect::<Vec<_>>()
    );

    // ---- compute ----
    let t = std::time::Instant::now();
    let shape = compute(&check);
    let compute_time = t.elapsed();
    let success = match &shape {
        RenderShape::Success(s) => s,
        RenderShape::Failure(f) => panic!("render compute failed: {:?}", f.diagnostics),
    };
    let components = success.components.len();
    let shape_edges = success.edges.len();

    // ---- layout ----
    let t = std::time::Instant::now();
    let positioned = layout(&shape, &LayoutConfig::default());
    let layout_time = t.elapsed();
    let cards = positioned.cards.len();
    let edges = positioned.edges.len();

    // ---- render ----
    let t = std::time::Instant::now();
    let svg = render(&positioned, &ThemeConfig::default());
    let render_time = t.elapsed();

    let total = compute_time + layout_time + render_time;
    let nodes = svg_node_count(&svg);

    eprintln!(
        "large-document render gate ({} declared persons → {cards} cards, {edges} positioned edges, {shape_edges} shape edges, {components} components):",
        declared_persons
    );
    eprintln!("  compute: {compute_time:?}");
    eprintln!("  layout:  {layout_time:?}");
    eprintln!("  render:  {render_time:?}");
    eprintln!("  total:   {total:?}");
    eprintln!("  svg:     {nodes} nodes, {} bytes", svg.len());

    // Profile-aware wall-clock ceiling. The LSP server ships release-built
    // (`release.yml` builds `kul-lsp --release`), so the *release* number is
    // the real per-edit cost a user feels; debug is what `cargo test` gates
    // by default and must absorb debug/CI variance. Per the perf.rs
    // convention we state the real observed target in a comment and assert a
    // generous (~5×) ceiling so a 2× regression still fires.
    //
    // Observed on a developer laptop for the 5,000-card fixture (per-stage:
    // `compute` dominates — it carries the parse + check + canonical
    // projection; `layout` and `render` are comparatively cheap):
    //   release: compute ≈ 28ms, layout ≈ 6ms, render ≈ 14ms, total ≈ 47ms
    //            → ceiling 250ms (~5×)
    //   debug:   compute ≈ 233ms, layout ≈ 12ms, render ≈ 27ms, total ≈ 272ms
    //            → ceiling 1500ms (~5×, absorbs CI/debug variance)
    let ceiling = if cfg!(debug_assertions) {
        std::time::Duration::from_millis(1_500)
    } else {
        std::time::Duration::from_millis(250)
    };
    assert!(
        total < ceiling,
        "large-document render budget exceeded: total {total:?} >= ceiling {ceiling:?}"
    );

    // SVG node count is a function of the fixture and the SVG schema, not the
    // build profile, so the same ceiling holds in debug and release. Observed
    // ≈ 19,166 nodes; the ceiling sits below 2× observed so a schema
    // regression that doubles nodes-per-element trips it, while leaving room
    // for modest legitimate growth.
    assert!(
        nodes < 30_000,
        "SVG node count {nodes} exceeds ceiling — nodes-per-element may have regressed"
    );
}
