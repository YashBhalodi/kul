//! Parent-graph cycle detection (rule 13).
//!
//! Iterative DFS with recursion-stack tracking. Each back-edge corresponds
//! to one detected cycle, so reporting once per back-edge gives "each cycle
//! exactly once" without further bookkeeping. Runs in O(V+E) over the
//! parent graph.

use std::collections::HashMap;

use crate::semantic::{ParentLink, ResolvedDocument};
use crate::span::ByteSpan;

#[derive(Debug, Clone)]
pub struct Cycle<'a> {
    /// Person ids on the cycle, in traversal order. The first id is the
    /// "first detected" person, and the last id appears as a parent of some
    /// node already in the prefix — closing the loop.
    pub members: Vec<&'a str>,
    /// `links[i]` is the link from `members[i]` to its child (i.e. to
    /// `members[i-1]`, with `members[0]`'s link wrapping to `members[last]`).
    /// Each link span anchors a "this child → that parent" arrow on the
    /// graph, used as related-info on the diagnostic.
    pub link_spans: Vec<ByteSpan>,
}

pub fn find_cycles<'a>(resolved: &ResolvedDocument<'a>) -> Vec<Cycle<'a>> {
    // Stable iteration order: walk persons in source order so diagnostics
    // are deterministic across runs.
    let order: Vec<&'a str> = resolved
        .document
        .statements
        .iter()
        .filter_map(|s| match s {
            crate::ast::Statement::Person(p) => Some(p.id.name.as_str()),
            _ => None,
        })
        .collect();

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }
    let mut color: HashMap<&'a str, Color> = HashMap::new();
    for &id in &order {
        color.insert(id, Color::White);
    }

    let mut cycles = Vec::new();

    // Iterative DFS using an explicit stack of (node, child-iterator-index,
    // children-cache). `path` mirrors the "currently on stack" set and
    // preserves the order needed to extract the cycle on a back-edge.
    for &start in &order {
        if color.get(start).copied() != Some(Color::White) {
            continue;
        }
        // Each stack frame: (node, parents_list, parent_idx).
        // `path_links[i]` = the parent-link span used to enter `path[i+1]`
        // — i.e. how we descended from path[i] to path[i+1].
        let mut path: Vec<&'a str> = Vec::new();
        let mut path_links: Vec<ByteSpan> = Vec::new();
        let mut frames: Vec<(&'a str, Vec<ParentLink<'a>>, usize)> = Vec::new();

        color.insert(start, Color::Gray);
        path.push(start);
        let parents = resolved.parents_of(start);
        frames.push((start, parents, 0));

        while let Some((node, parents, idx)) = frames.last_mut() {
            if *idx >= parents.len() {
                color.insert(*node, Color::Black);
                path.pop();
                if !path_links.is_empty() {
                    path_links.pop();
                }
                frames.pop();
                continue;
            }
            let link = parents[*idx].clone();
            *idx += 1;
            let parent = link.parent_id;
            match color.get(parent).copied().unwrap_or(Color::White) {
                Color::White => {
                    color.insert(parent, Color::Gray);
                    path.push(parent);
                    path_links.push(link.link_span);
                    let next_parents = resolved.parents_of(parent);
                    frames.push((parent, next_parents, 0));
                }
                Color::Gray => {
                    // Back-edge: parent is an ancestor of node. The cycle
                    // consists of `path[parent_idx..]` plus the closing
                    // edge `node → parent`.
                    let parent_idx = path
                        .iter()
                        .position(|&p| p == parent)
                        .expect("Gray node must be on the path");
                    let members: Vec<&'a str> = path[parent_idx..].to_vec();
                    let mut link_spans: Vec<ByteSpan> = path_links[parent_idx..].to_vec();
                    // The closing back-edge is the link we just traversed.
                    link_spans.push(link.link_span);
                    cycles.push(Cycle {
                        members,
                        link_spans,
                    });
                }
                Color::Black => {
                    // Finished — already explored, no cycle through here.
                }
            }
        }
    }

    cycles
}
