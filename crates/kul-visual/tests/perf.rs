//! Performance budget gate for the canonical render pipeline: the
//! `compute` → `layout` → `render` chain over a ~5,000-card project. The
//! ceilings are ~5× the observed time, so 2× regressions still fire while
//! CI runner variance is absorbed. Each stage is timed on its own, so the
//! gate reaches the pinned pipeline crates directly rather than through the
//! `render_from_check` facade.

use std::fmt::Write as _;

use kul_layout::{LayoutConfig, layout};
use kul_render::{RenderShape, compute};
use kul_svg::{ThemeConfig, render};

fn emit_person(out: &mut String, pid: usize, gender: &str, born: u32, birth: Option<usize>) {
    let _ = writeln!(
        out,
        "person p{pid} name:\"Person {pid}\" gender:{gender} born:{born}"
    );
    if let Some(m) = birth {
        let _ = writeln!(out, "  birth m{m}");
    }
}

/// How one family tree grows: generations below the founder, children per
/// marriage, and which children marry in turn.
#[derive(Clone, Copy)]
struct Shape {
    depth: usize,
    children_per: usize,
    marry_every: usize,
}

/// The next free person and marriage numbers, so ids stay unique.
#[derive(Default)]
struct Ids {
    persons: usize,
    marriages: usize,
}

/// Grow a family subtree from `spouse_a`: marry in a fresh spouse, declare
/// the marriage, emit `children_per` children referencing it, and recurse on
/// every `marry_every`-th child until `depth` runs out. All references point
/// backwards, so the output validates without forward resolution.
fn grow_family(out: &mut String, ids: &mut Ids, spouse_a: usize, shape: Shape, born: u32) {
    let spouse_b = ids.persons;
    ids.persons += 1;
    let gender = if spouse_b % 2 == 0 { "female" } else { "male" };
    emit_person(out, spouse_b, gender, born.saturating_sub(2), None);

    let m = ids.marriages;
    ids.marriages += 1;
    let _ = writeln!(out, "marriage m{m} p{spouse_a} p{spouse_b} start:{born}");

    for i in 0..shape.children_per {
        let child = ids.persons;
        ids.persons += 1;
        let child_gender = if child % 2 == 0 { "female" } else { "male" };
        emit_person(out, child, child_gender, born + 25, Some(m));
        if shape.depth > 1 && i % shape.marry_every == 0 {
            let below = Shape {
                depth: shape.depth - 1,
                ..shape
            };
            grow_family(out, ids, child, below, born + 26);
        }
    }
}

/// Generate a ~`target_cards`-card Kul source: mixed-depth family trees
/// plus singleton documentation roots. Built in-test so the `examples/`
/// corpus stays untouched. Returns the source and the declared-person count.
fn generate_large_document(target_cards: usize) -> (String, usize) {
    let mut out = String::new();
    let mut ids = Ids::default();

    let tree_budget = target_cards - target_cards / 10;

    let shapes =
        [(1, 3, 1), (2, 3, 2), (3, 2, 1), (4, 2, 2)].map(|(depth, children_per, marry_every)| {
            Shape {
                depth,
                children_per,
                marry_every,
            }
        });
    let mut shape_idx = 0usize;
    let mut year = 1900u32;

    while ids.persons < tree_budget {
        let shape = shapes[shape_idx % shapes.len()];
        shape_idx += 1;
        let founder = ids.persons;
        ids.persons += 1;
        emit_person(&mut out, founder, "male", year, None);
        grow_family(&mut out, &mut ids, founder, shape, year + 24);
        year += 7;
        if year > 1980 {
            year = 1900;
        }
    }

    while ids.persons < target_cards {
        let s = ids.persons;
        ids.persons += 1;
        let gender = if s % 2 == 0 { "female" } else { "male" };
        emit_person(&mut out, s, gender, 1960, None);
    }

    (out, ids.persons)
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
    let check = kul_core::check(&inputs);
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
    let positioned = layout(success, &LayoutConfig::default());
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
