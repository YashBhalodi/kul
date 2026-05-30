//! Layout engine: buffers a region of cell-streams, computes per-group
//! column widths, and emits aligned text.

use std::collections::HashMap;

use crate::ast::{MarriageStmt, PersonStmt, Statement};

use super::cells::{
    Cell, CellKind, GroupKey, KindTag, RegionItem, build_marriage_cells, build_person_cells,
    build_sub_cells, canonical_columns, collect_sub_refs,
};

pub(super) struct Emitter {
    out: String,
    region: Vec<RegionItem>,
    /// Per-region scope id for a person's sub-statements; reset at each
    /// region flush.
    next_parent_id: u32,
}

impl Emitter {
    pub(super) fn new() -> Self {
        Self {
            out: String::new(),
            region: Vec::new(),
            next_parent_id: 0,
        }
    }

    pub(super) fn allocate_parent_id(&mut self) -> u32 {
        let id = self.next_parent_id;
        self.next_parent_id += 1;
        id
    }

    pub(super) fn emit_top_level(&mut self, indent: usize, kind: KindTag, cells: Vec<Cell>) {
        let group = GroupKey {
            indent,
            kind,
            parent: None,
        };
        self.region.push(RegionItem::Aligned {
            indent,
            cells,
            group,
        });
    }

    pub(super) fn emit_sub(
        &mut self,
        parent_id: u32,
        indent: usize,
        kind: KindTag,
        cells: Vec<Cell>,
    ) {
        let group = GroupKey {
            indent,
            kind,
            parent: Some(parent_id),
        };
        self.region.push(RegionItem::Aligned {
            indent,
            cells,
            group,
        });
    }

    pub(super) fn emit_comment(&mut self, indent: usize, text: String) {
        self.region.push(RegionItem::Comment { indent, text });
    }

    /// Compute per-group widths and flush the buffered region. `None` in
    /// a width vec marks a column no line carries — the renderer emits
    /// no placeholder for it.
    pub(super) fn end_region(&mut self) {
        if self.region.is_empty() {
            return;
        }
        let mut widths: HashMap<GroupKey, Vec<Option<usize>>> = HashMap::new();
        for item in &self.region {
            if let RegionItem::Aligned { cells, group, .. } = item {
                let canonical = canonical_columns(group.kind);
                let entry = widths
                    .entry(*group)
                    .or_insert_with(|| vec![None; canonical.len()]);
                for cell in cells {
                    let slot = &mut entry[cell.col as usize];
                    let w = cell.text.chars().count();
                    *slot = Some(slot.unwrap_or(0).max(w));
                }
            }
        }
        let items = std::mem::take(&mut self.region);
        for item in items {
            match item {
                RegionItem::Aligned {
                    indent,
                    cells,
                    group,
                } => {
                    let cols = widths.get(&group).expect("group widths");
                    emit_aligned_line(indent, group.kind, &cells, cols, &mut self.out);
                }
                RegionItem::Comment { indent, text } => {
                    for _ in 0..indent {
                        self.out.push(' ');
                    }
                    self.out.push_str(&text);
                    self.out.push('\n');
                }
            }
        }
        self.next_parent_id = 0;
    }

    pub(super) fn finish(mut self) -> String {
        self.end_region();
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out
    }

    /// Append a blank-line separator (ADR-0004 rule 6: never at file start).
    pub(super) fn append_separator(&mut self) {
        if !self.out.is_empty() {
            self.out.push('\n');
        }
    }

    pub(super) fn emit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Person(p) => self.emit_person(p),
            Statement::Marriage(m) => self.emit_marriage(m),
        }
    }

    fn emit_person(&mut self, p: &PersonStmt) {
        self.emit_top_level(0, KindTag::Person, build_person_cells(p, None));
        let subs = collect_sub_refs(p);
        if subs.is_empty() {
            return;
        }
        let parent = self.allocate_parent_id();
        for sub in &subs {
            let (kind, cells) = build_sub_cells(sub, None);
            self.emit_sub(parent, 2, kind, cells);
        }
    }

    fn emit_marriage(&mut self, m: &MarriageStmt) {
        self.emit_top_level(0, KindTag::Marriage, build_marriage_cells(m, None));
    }
}

/// Render one line of an alignment group.
///
/// Walks the canonical column sequence for `kind`. Each present column
/// either emits the line's cell (padded, or unpadded if last), or emits
/// whitespace placeholder if the line lacks the cell but has more cells
/// to the right. Stops at the last actual cell — no trailing whitespace.
///
/// Inter-column separator is decided by adjacent canonical `CellKind`s:
/// single space between structurals, double space before fields/comments.
fn emit_aligned_line(
    indent: usize,
    kind: KindTag,
    cells: &[Cell],
    widths: &[Option<usize>],
    out: &mut String,
) {
    let canonical = canonical_columns(kind);
    debug_assert_eq!(canonical.len(), widths.len());
    for _ in 0..indent {
        out.push(' ');
    }

    // Stop after this column.
    let last_col = cells
        .last()
        .expect("a line always has at least the keyword cell")
        .col;

    let mut prev_col: Option<u8> = None;
    for (col_idx, slot_width) in widths.iter().enumerate() {
        let col_idx = col_idx as u8;
        if col_idx > last_col {
            break;
        }
        let Some(width) = *slot_width else {
            continue;
        };
        if let Some(prev) = prev_col {
            match separator_between(canonical[prev as usize], canonical[col_idx as usize]) {
                Sep::Single => out.push(' '),
                Sep::Double => out.push_str("  "),
            }
        }
        // char count, not byte length: non-ASCII identifiers count one
        // column per Unicode scalar. CJK display-width is out of scope.
        if let Some(cell) = cells.iter().find(|c| c.col == col_idx) {
            out.push_str(&cell.text);
            if col_idx != last_col {
                let pad = width.saturating_sub(cell.text.chars().count());
                for _ in 0..pad {
                    out.push(' ');
                }
            }
        } else {
            for _ in 0..width {
                out.push(' ');
            }
        }
        prev_col = Some(col_idx);
    }
    out.push('\n');
}

#[derive(Clone, Copy)]
enum Sep {
    Single,
    Double,
}

fn separator_between(prev: CellKind, next: CellKind) -> Sep {
    // Single space between structurals; double space before fields/comments.
    match next {
        CellKind::Field(_) | CellKind::Comment => Sep::Double,
        CellKind::Keyword | CellKind::Positional | CellKind::Reference => match prev {
            CellKind::Keyword | CellKind::Positional | CellKind::Reference => Sep::Single,
            // Field/comment → structural is not canonical; default safely.
            CellKind::Field(_) | CellKind::Comment => Sep::Single,
        },
    }
}
