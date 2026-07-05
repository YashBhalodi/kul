//! Query-engine performance budget gates (issue #261, PRD 0005).
//!
//! Perf budgets are **tests, not benchmarks** (docs/testing.md): they run in
//! every `cargo nextest`, so an interactive-latency regression fails loudly at
//! PR time rather than hiding until someone remembers to run a bench. Each
//! ceiling is generous (~5× the real target) so CI/runner variance never
//! flakes the gate while a 2× regression still trips it; the real target lives
//! in a comment.
//!
//! The workload is a **deterministic, seed-free ~10k-person synthetic
//! project** (`generate_corpus`) built in-test so the `examples/` snapshots
//! stay untouched. It is constructed, never randomised — the same bytes every
//! run — so both the budget and any future snapshot over it are reproducible.
//! Its shape is the real shape of the problem: a ~12-generation dynasty grown
//! breadth-first at a branching factor of two (the factor a ~10k-person /
//! 12-generation history actually implies) so every generation is fully
//! populated, carrying the structural hazards traversal cost is really paid on
//! — a few polygamous households, adoptions (including adoption-into-relatives,
//! which puts a genuine cycle in the relation graph), divorce-and-remarriage
//! half-sibling forks — plus a second, fully disconnected dynasty so the
//! connectivity answer is measured too.
//!
//! The engine runs **on-demand over `ResolvedDocument` with per-invocation
//! structures only — no dedicated query indices, no cross-query caching**
//! (ADR-0029). This gate is the guard on that constraint: if these operations
//! ever fall out of budget, the fix is a faster on-demand traversal, not a
//! cache (that trade-off needs a human decision — see the issue).

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;
use kul_core::query::{PersonField, SortDirection};
use kul_core::query::{
    Predicate, Query, ResolveConfig, SortSpec, ancestors_of, cousins_of, descendants_of, resolve,
    run_query,
};

/// The landmark ids `generate_corpus` pins so the budget operations run at
/// realistic depths — a deep leaf, a fertile root, a mid-tree person, an
/// ancestor ~8 generations up, and a person in a second disconnected
/// component. Their existence at these depths is what makes the measured cost
/// representative rather than a lucky shallow walk.
struct Landmarks {
    declared_persons: usize,
    /// Founder of the primary dynasty — deep descendants below it.
    root: String,
    /// Leaf at the bottom of the ~12-generation heir line.
    deep_leaf: String,
    /// Person mid-way down the heir line (second-cousin neighbourhood above
    /// and descendants below).
    mid: String,
    /// An ancestor ~8 generations up from `deep_leaf` — the far end of the
    /// "pair ~8 generations apart" resolution.
    lineal_ancestor: String,
    /// A person in the *second*, disconnected dynasty — for the cross-component
    /// connectivity answer.
    other_component: String,
    /// Any valid id, for the `person(id)` detail-lookup budget.
    detail: String,
}

/// Builds the deterministic synthetic project source and the landmark ids.
struct Builder {
    out: String,
    /// Next person counter (`p{pc}`).
    pc: usize,
    /// Next marriage counter (`m{mc}`).
    mc: usize,
    /// The deepest generation the heir line reaches.
    max_gen: usize,
    base_born: u32,
    /// The heir person at each generation (`spine[g]`).
    spine: Vec<String>,
    /// Deterministic counters that place the structural hazards without any
    /// randomness (snapshots and budgets must be reproducible).
    adopt_counter: usize,
    polygamy_counter: usize,
    remarriage_counter: usize,
}

impl Builder {
    fn new(max_gen: usize, base_born: u32) -> Self {
        Builder {
            out: String::new(),
            pc: 0,
            mc: 0,
            max_gen,
            base_born,
            spine: Vec::new(),
            adopt_counter: 0,
            polygamy_counter: 0,
            remarriage_counter: 0,
        }
    }

    /// Emit a bare person, returning its id. Gender alternates by counter so
    /// marriage hosts and spouses differ without any randomness.
    fn person(&mut self, born: u32) -> String {
        let id = format!("p{}", self.pc);
        let gender = if self.pc % 2 == 0 { "female" } else { "male" };
        self.pc += 1;
        let _ = writeln!(
            self.out,
            "person {id} name:\"P{id}\" gender:{gender} born:{born}"
        );
        id
    }

    /// Emit a person born into `birth_m` (a `birth` sub-statement).
    fn child(&mut self, born: u32, birth_m: &str) -> String {
        let id = self.person(born);
        let _ = writeln!(self.out, "  birth {birth_m}");
        id
    }

    /// Emit an un-ended marriage `host` × `spouse`, returning its id. `host`
    /// is spouse_a — the position a polygamous hub must occupy (R14).
    fn marry(&mut self, host: &str, spouse: &str, start: u32) -> String {
        let id = format!("m{}", self.mc);
        self.mc += 1;
        let _ = writeln!(self.out, "marriage {id} {host} {spouse} start:{start}");
        id
    }

    /// Emit a divorced marriage — the first half of a remarriage fork.
    fn marry_divorced(&mut self, host: &str, spouse: &str, start: u32, end: u32) -> String {
        let id = format!("m{}", self.mc);
        self.mc += 1;
        let _ = writeln!(
            self.out,
            "marriage {id} {host} {spouse} start:{start} end:{end} end_reason:divorce"
        );
        id
    }

    /// Add an `adoption` sub-statement to the just-emitted person.
    fn adopt_into(&mut self, m: &str, start: u32) {
        let _ = writeln!(self.out, "  adoption {m} start:{start}");
    }

    fn born_at(&self, generation: usize) -> u32 {
        self.base_born + (generation as u32) * 25
    }

    /// Grow the dynasty rooted at `parent_m` (whose spouses are generation
    /// `generation - 1`). `grandparent_m` is the marriage two generations up, used to
    /// place adoption-into-relatives (a grandparent adopting a grandchild — a
    /// real pattern, and one that seeds a cycle in the relation graph). The
    /// heir line (`on_spine`, child index 0) is grown depth-first and is never
    /// budget-gated, so the ~12-generation spine is guaranteed even as cadet
    /// breadth fills the person budget.
    /// Grow one connected dynasty **breadth-first** from `root_marriage` until
    /// the running person count reaches `until`. BFS (not DFS) is load-bearing:
    /// it distributes the budget evenly across generations, so every generation
    /// is fully populated and cousins / second-cousins exist at every level —
    /// the mid-tree collateral queries have real answers, and a deep leaf sits
    /// in a genuinely dense neighbourhood (the realistic resolution cost).
    ///
    /// When `track_spine`, the first child of each spine marriage is recorded as
    /// `spine[gen]`, and that heir line is grown to `max_gen` even after the
    /// budget is spent — guaranteeing the full-depth ~12-generation lineage.
    /// Each couple has three children; the first two marry (branching two, so
    /// the tree reaches ~12 generations within a ~10k budget), the third is a
    /// non-marrying leaf. Structural hazards are woven in deterministically.
    fn grow_bfs(&mut self, root_marriage: String, until: usize, track_spine: bool) {
        // Queue items: (marriage, grandparent marriage, generation, on-spine).
        let mut queue: VecDeque<(String, Option<String>, usize, bool)> = VecDeque::new();
        queue.push_back((root_marriage, None, 1, track_spine));

        while let Some((parent_m, grandparent_m, generation, on_spine)) = queue.pop_front() {
            if generation > self.max_gen {
                continue;
            }
            let born = self.born_at(generation);
            for i in 0..3 {
                // Off-spine children stop once the budget is spent; the spine
                // heir (i == 0 on the spine) is always created.
                let is_heir = i == 0 && on_spine;
                if !is_heir && self.pc >= until {
                    break;
                }
                let child = self.child(born, &parent_m);

                // Adoption-into-relatives: the grandparents occasionally adopt
                // the grandchild — the child is then both their descendant (via
                // birth) and their adoptive child, a deliberate cycle for the
                // traversal guard to survive.
                if let Some(gm) = &grandparent_m {
                    if self.adopt_counter % 900 == 0 {
                        self.adopt_into(gm, born + 5);
                    }
                    self.adopt_counter += 1;
                }

                if is_heir {
                    self.spine.push(child.clone());
                }

                // Only the first two children marry (branching two); the third
                // is a leaf. The spine heir always marries.
                let marries = generation < self.max_gen && (is_heir || (i < 2 && self.pc < until));
                if marries {
                    let child_m = self.marry_child(&child, born, until);
                    queue.push_back((child_m, Some(parent_m.clone()), generation + 1, is_heir));
                }
            }
        }
    }

    /// Marry `child` in and return the marriage that carries its children,
    /// weaving in the remarriage and polygamy hazards deterministically. The
    /// returned marriage is the one the next generation is born into.
    fn marry_child(&mut self, child: &str, born: u32, until: usize) -> String {
        let spouse = self.person(born.saturating_sub(2));

        // Remarriage fork: a divorced first marriage plus a fresh one, so the
        // children split into half-siblings across the two unions. The fresh
        // marriage carries the ongoing line.
        self.remarriage_counter += 1;
        if self.remarriage_counter % 650 == 0 && self.pc + 4 < until {
            let first = self.marry_divorced(child, &spouse, born + 22, born + 30);
            let _ = self.child(born + 24, &first);
            let spouse2 = self.person(born);
            let second = self.marry(child, &spouse2, born + 32);
            let _ = self.child(born + 34, &second);
            return second;
        }

        let m = self.marry(child, &spouse, born + 22);

        // Polygamy hub: a second concurrent, un-ended marriage with `child` as
        // host in both (R14) — half-siblings across co-spouses.
        self.polygamy_counter += 1;
        if self.polygamy_counter % 500 == 0 && self.pc + 3 < until {
            let cowife = self.person(born + 1);
            let m2 = self.marry(child, &cowife, born + 24);
            let _ = self.child(born + 26, &m2);
        }

        m
    }

    /// Grow one independent dynasty rooted at a fresh founding couple, stopping
    /// at `until` declared persons. Returns the founder id. Shares no ids with
    /// any prior component, so it is a genuinely disconnected component.
    fn grow_component(&mut self, until: usize) -> String {
        let born = self.base_born;
        let founder = self.person(born);
        let spouse = self.person(born.saturating_sub(2));
        let m = self.marry(&founder, &spouse, born + 22);
        self.grow_bfs(m, until, false);
        founder
    }
}

/// Build the deterministic ~10k-person project and its landmark ids.
fn generate_corpus() -> (String, Landmarks) {
    let max_gen = 12;
    // Primary dynasty target; the second component and any top-up singletons
    // bring the declared total to ~10k.
    let primary_budget = 9_400;
    let mut b = Builder::new(max_gen, 1700);

    // Primary dynasty: founding couple, then the breadth-first growth.
    let founder = b.person(b.base_born);
    b.spine.push(founder.clone());
    let spouse = b.person(b.base_born.saturating_sub(2));
    let m0 = b.marry(&founder, &spouse, b.base_born + 22);
    b.grow_bfs(m0, primary_budget, true);

    // Second, disconnected dynasty (~450 persons) for the connectivity answer.
    let other_founder = b.grow_component(b.pc + 450);

    // Top up with singleton documentation persons to land near 10k exactly.
    while b.pc < 10_000 {
        let _ = b.person(1900);
    }

    let deep_leaf = b.spine[max_gen].clone();
    let landmarks = Landmarks {
        declared_persons: b.pc,
        root: founder,
        mid: b.spine[max_gen / 2].clone(),
        lineal_ancestor: b.spine[4].clone(),
        deep_leaf,
        other_component: other_founder,
        detail: b.spine[6].clone(),
    };
    (b.out, landmarks)
}

/// One measured operation: its label, the closure exercising the engine, and
/// the elapsed time. The closure returns a small witness (a count) so the
/// optimiser can't elide the work.
fn measure(label: &str, op: impl FnOnce() -> usize) -> Duration {
    let start = Instant::now();
    let witness = op();
    let elapsed = start.elapsed();
    eprintln!("  {label:<28} {elapsed:>10.2?}  (n={witness})");
    elapsed
}

#[test]
fn ten_thousand_person_query_operations_under_budget() {
    let (source, marks) = generate_corpus();

    let inputs = vec![InputFile::new("large.kul", source.as_str())];
    let check = kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs);
    assert!(
        check.diagnostics.is_empty(),
        "generated fixture must validate cleanly; got {} diagnostics: {:?}",
        check.diagnostics.len(),
        check.diagnostics.iter().take(5).collect::<Vec<_>>()
    );
    assert!(
        (9_500..=10_500).contains(&marks.declared_persons),
        "corpus should be ~10k persons, got {}",
        marks.declared_persons
    );

    // The structural hazards must actually be present — a clean binary tree
    // would measure the wrong thing. Divorce/remarriage and adoption show up as
    // markers in the source; polygamy hubs show up as persons hosting ≥2
    // un-ended marriages (R14-valid because the hub is always spouse_a); the
    // cross-component `resolve` below is the ≥2-disconnected-components witness.
    assert!(
        source.contains("end_reason:divorce"),
        "corpus must contain remarriage (divorce) hazards"
    );
    assert!(
        source.contains("\n  adoption "),
        "corpus must contain adoption hazards (incl. adoption-into-relatives)"
    );
    let mut hosting: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for m in check.resolved().marriages() {
        if m.end().is_none() {
            *hosting.entry(m.spouse_a.name.as_str()).or_default() += 1;
        }
    }
    assert!(
        hosting.values().any(|&n| n >= 2),
        "corpus must contain at least one polygamous household"
    );

    let resolved = check.resolved();
    let cfg = ResolveConfig::default();

    eprintln!(
        "10k-person query budget ({} persons, {} generations):",
        marks.declared_persons, 12
    );

    // The seven representative operations, each timed independently. Real
    // interactive target: **< ~50 ms each** (PRD 0005). On a dev laptop the
    // whole set lands there with margin in the shipped (release) profile —
    // kin-set/filter/lookup ≈ 1–3 ms, and the heaviest, `resolve` (which
    // enumerates *every* way two people are related over the cap-bounded
    // neighbourhood), ≈ 16–17 ms. No query indices or cross-query caches are
    // used to reach this — the engine runs on-demand over `ResolvedDocument`
    // with per-invocation structures only (ADR-0029); the budget is met within
    // that constraint, so no cache was introduced.
    let budgets = [
        measure("ancestors_of(depth 5)", || {
            ancestors_of(resolved, &marks.deep_leaf, Some(5))
                .expect("known id")
                .len()
        }),
        measure("descendants_of(depth 5)", || {
            descendants_of(resolved, &marks.root, Some(5))
                .expect("known id")
                .len()
        }),
        measure("cousins_of(degree 2)", || {
            cousins_of(resolved, &marks.mid, 2, 0)
                .expect("known id")
                .len()
        }),
        measure("resolve(~8 gens apart)", || {
            // A pair ~8 generations apart at the default cap: the lineal tie
            // plus whatever else the neighbourhood holds.
            resolve(resolved, &marks.deep_leaf, &marks.lineal_ancestor, &cfg)
                .expect("known ids")
                .relationships
                .len()
        }),
        measure("resolve(cross-component)", || {
            // Connectivity answer: different components ⇒ Disconnected. Must
            // also be fast — a bounded reachability verdict, not the full cap.
            let r = resolve(resolved, &marks.deep_leaf, &marks.other_component, &cfg)
                .expect("known ids");
            assert!(
                r.relationships.is_empty() && r.empty_reason.is_some(),
                "cross-component pair must be disconnected"
            );
            0
        }),
        measure("allPersons where+sort+count", || {
            // Date-range `where` + `sort born` + `count` over the whole corpus.
            let query = Query::all_persons()
                .filtered(Predicate::Gte {
                    field: PersonField::Born,
                    value: "1850".to_string(),
                })
                .filtered(Predicate::Lte {
                    field: PersonField::Born,
                    value: "1950".to_string(),
                })
                .sorted(SortSpec {
                    field: PersonField::Born,
                    direction: SortDirection::Asc,
                });
            match run_query(resolved, &query).expect("valid query") {
                kul_core::query::QueryResult::PersonIds { person_ids } => person_ids.len(),
                other => panic!("expected person ids, got {other:?}"),
            }
        }),
        measure("person(id) lookup", || {
            usize::from(kul_core::query::person(resolved, &marks.detail).is_some())
        }),
    ];

    // Real target 50 ms; the ceiling sits ~5× that so CI/debug variance never
    // flakes the gate, but a 2× regression still fires (docs/testing.md). Debug
    // runs ~4–5× slower than release, so the debug ceiling is higher. Every
    // operation shares one ceiling — they are all "one interactive query".
    let ceiling = if cfg!(debug_assertions) {
        Duration::from_millis(400)
    } else {
        Duration::from_millis(250)
    };
    let worst = budgets.iter().copied().max().unwrap();
    assert!(
        worst < ceiling,
        "a query operation exceeded the interactive budget: worst {worst:?} >= ceiling {ceiling:?}"
    );
}
