//! Parent-graph cycle detection (rule 13).
//!
//! Iterative DFS with white/gray/black coloring. Each back-edge
//! corresponds to one detected cycle, so reporting once per back-edge
//! gives "each cycle exactly once" without further bookkeeping. Runs in
//! O(V+E) over the parent graph.
//!
//! All graph traversal goes through [`ResolvedDocument::parents_of`] —
//! this module never enumerates `document.statements` itself.
//!
//! The parent graph is project-wide (per ADR-0015): a cycle that spans
//! two files is detected just like a cycle inside one file.

use std::collections::HashMap;

use crate::ast::PersonStmt;
use crate::semantic::{ParentLink, ResolvedDocument};
use crate::span::{ByteSpan, FileId};

#[derive(Debug, Clone)]
pub struct CycleLink {
    pub span: ByteSpan,
    pub file: FileId,
}

#[derive(Debug, Clone)]
pub struct Cycle<'a> {
    /// Persons on the cycle in traversal order. The first entry is the
    /// "first detected" node; the closing back-edge runs from the last
    /// member back to the first.
    pub members: Vec<&'a PersonStmt>,
    /// Every parent-link forming this cycle, including the closing
    /// back-edge — one per arrow on the path. Each link carries its
    /// source span and the file containing that span (the child's
    /// owning file, which may differ from the parent's under
    /// project-wide resolution).
    pub link_spans: Vec<CycleLink>,
}

/// Find every parenthood cycle reachable from any person in the project.
/// The parent graph spans every `.kul` file (per ADR-0015); a cycle that
/// crosses files is detected the same way as a within-file cycle.
pub fn find_cycles(resolved: &ResolvedDocument) -> Vec<Cycle<'_>> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    let order: Vec<&PersonStmt> = resolved.persons().collect();
    let mut color: HashMap<&str, Color> = order
        .iter()
        .map(|p| (p.id.name.as_str(), Color::White))
        .collect();

    let mut cycles = Vec::new();

    for &start in &order {
        if color.get(start.id.name.as_str()).copied() != Some(Color::White) {
            continue;
        }

        let mut path: Vec<&PersonStmt> = Vec::new();
        let mut path_links: Vec<CycleLink> = Vec::new();
        let mut frames: Vec<(&PersonStmt, Vec<ParentLink<'_>>, usize)> = Vec::new();

        color.insert(start.id.name.as_str(), Color::Gray);
        path.push(start);
        frames.push((start, resolved.parents_of(start), 0));

        while let Some((node, parents, idx)) = frames.last_mut() {
            if *idx >= parents.len() {
                color.insert(node.id.name.as_str(), Color::Black);
                path.pop();
                if !path_links.is_empty() {
                    path_links.pop();
                }
                frames.pop();
                continue;
            }
            let link = parents[*idx].clone();
            *idx += 1;
            let parent = link.parent;
            let parent_color = color
                .get(parent.id.name.as_str())
                .copied()
                .unwrap_or(Color::White);
            match parent_color {
                Color::White => {
                    color.insert(parent.id.name.as_str(), Color::Gray);
                    path.push(parent);
                    path_links.push(CycleLink {
                        span: link.link_span,
                        file: link.link_file,
                    });
                    frames.push((parent, resolved.parents_of(parent), 0));
                }
                Color::Gray => {
                    let parent_idx = path
                        .iter()
                        .position(|&p| std::ptr::eq(p, parent))
                        .expect("Gray node must be on the path");
                    let members: Vec<&PersonStmt> = path[parent_idx..].to_vec();
                    let mut link_spans: Vec<CycleLink> = path_links[parent_idx..].to_vec();
                    link_spans.push(CycleLink {
                        span: link.link_span,
                        file: link.link_file,
                    });
                    cycles.push(Cycle {
                        members,
                        link_spans,
                    });
                }
                Color::Black => {}
            }
        }
    }

    cycles
}
