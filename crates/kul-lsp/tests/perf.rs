//! Performance budget gates: each test asserts a generous (~5×) ceiling
//! around the real target so 2× regressions still fire while absorbing CI
//! runner variance. No stdio framing — measures the language pipeline
//! through the public `kul_lsp` surface against `kul_core::check`.

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

/// Hand-build a `ProjectEntry` from already-checked inputs, bypassing
/// `Documents` so the perf measurement excludes disk I/O and async.
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
    // PRD target 100ms; 500ms ceiling absorbs CI/debug variance.
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "1000-statement budget exceeded: {elapsed:?}"
    );
}

#[test]
fn ten_files_of_one_hundred_statements_under_budget() {
    // Per-project budget (ADR-0015). Regressing faster than the
    // single-file 1000-statement test above signals a per-file cost
    // in the resolver.
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
    // Translate per file to mirror the LSP slice's project-wide
    // diagnostic broadcast.
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

fn emit_person(out: &mut String, pid: usize, gender: &str, born: u32, birth: Option<usize>) {
    let _ = writeln!(
        out,
        "person p{pid} name:\"Person {pid}\" gender:{gender} born:{born}"
    );
    if let Some(m) = birth {
        let _ = writeln!(out, "  birth m{m}");
    }
}

/// Grow a family subtree from `spouse_a`: marry in a fresh spouse,
/// declare the marriage, emit `children_per` children referencing it,
/// recurse on every `marry_every`-th child until `depth` runs out. All
/// references point backwards so the output validates without
/// forward-resolution. `pc`/`mc` thread through by `&mut` to keep ids
/// globally unique.
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

/// Generate a ~`target_cards`-card Kul source: mixed-depth family trees
/// plus singleton documentation roots. Built in-test so the
/// `examples/` corpus snapshots stay untouched. Returns the source and
/// the declared-person count.
fn generate_large_document(target_cards: usize) -> (String, usize) {
    let mut out = String::new();
    let mut pc = 0usize;
    let mut mc = 0usize;

    let tree_budget = target_cards - target_cards / 10;

    let shapes = [
        // (depth, children_per, marry_every)
        (1, 3, 1),
        (2, 3, 2),
        (3, 2, 1),
        (4, 2, 2),
    ];
    let mut shape_idx = 0usize;
    let mut year = 1900u32;

    while pc < tree_budget {
        let (depth, children_per, marry_every) = shapes[shape_idx % shapes.len()];
        shape_idx += 1;
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
        year += 7;
        if year > 1980 {
            year = 1900;
        }
    }

    while pc < target_cards {
        let s = pc;
        pc += 1;
        let gender = if s % 2 == 0 { "female" } else { "male" };
        emit_person(&mut out, s, gender, 1960, None);
    }

    (out, pc)
}

/// SVG element count as a browser-DOM cost proxy.
fn svg_node_count(svg: &str) -> usize {
    svg.matches('<').count() - svg.matches("</").count()
}

/// Large-document render gate: times the full `compute` → `layout` →
/// `render` chain over a ~5,000-card fixture, with per-profile wall-clock
/// and profile-independent SVG node-count ceilings. Logs the per-stage
/// breakdown so a future optimisation follow-up knows which stage
/// dominates without re-instrumenting.
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

    let t = std::time::Instant::now();
    let shape = compute(&check);
    let compute_time = t.elapsed();
    let success = match &shape {
        RenderShape::Success(s) => s,
        RenderShape::Failure(f) => panic!("render compute failed: {:?}", f.diagnostics),
    };
    let components = success.components.len();
    let shape_edges = success.edges.len();

    let t = std::time::Instant::now();
    let positioned = layout(&shape, &LayoutConfig::default());
    let layout_time = t.elapsed();
    let cards = positioned.cards.len();
    let edges = positioned.edges.len();

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

    // Observed on a developer laptop (compute dominates):
    //   release total ≈ 47ms → ceiling 250ms (~5×)
    //   debug   total ≈ 272ms → ceiling 1500ms (~5×, CI/debug variance)
    let ceiling = if cfg!(debug_assertions) {
        std::time::Duration::from_millis(1_500)
    } else {
        std::time::Duration::from_millis(250)
    };
    assert!(
        total < ceiling,
        "large-document render budget exceeded: total {total:?} >= ceiling {ceiling:?}"
    );

    // Profile-independent; observed ≈ 19,166 nodes. Ceiling sits below
    // 2× observed so a doubling of nodes-per-element trips it.
    assert!(
        nodes < 30_000,
        "SVG node count {nodes} exceeds ceiling — nodes-per-element may have regressed"
    );
}
