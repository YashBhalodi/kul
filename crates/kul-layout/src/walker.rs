//! Buchheim et al. (2002) O(n) Reingold–Tilford–Walker tidy-tree
//! algorithm.
//!
//! Pure algorithm — no kul vocabulary lives here. The input is a
//! generic tree of [`InputNode`]s carrying each node's natural width
//! and the indices of its children; the output is the same tree with
//! a final x-coordinate per node.
//!
//! ## Algorithm shape
//!
//! Two walks:
//!
//! 1. **First walk, post-order ([`first_walk`]).** Assigns each
//!    interior node a *preliminary* x that centers it above its
//!    children, then resolves sibling-subtree collisions by shifting
//!    the right subtree and recording a per-subtree `modifier` that
//!    propagates the shift to descendants in the second pass.
//! 2. **Second walk, pre-order ([`second_walk`]).** Accumulates the
//!    modifiers down the tree to compute each node's final absolute
//!    x.
//!
//! The collision-avoidance uses Buchheim's thread + ancestor + change
//! / shift machinery to amortise the contour walks; the implementation
//! sticks close to the paper's pseudocode for ease of review.
//!
//! ## Why bother
//!
//! v1's tracer (`examples/03-three-generations/`) does not trigger
//! sibling-subtree overlap. Walker is in place anyway so the next
//! follow-up issue that *does* introduce overlap (multi-branch
//! dynasties under source order, cousin-marriage under the absorb rule) lands without
//! algorithmic work — the adapter just hands a wider tree in.

/// One node in the layout tree the algorithm consumes.
///
/// The caller (the adapter) builds a `Vec<InputNode>` flat-indexed
/// representation: child indices reference positions in the same
/// vector. Root nodes have no parent; sibling order is the order they
/// appear in their parent's `children` slice.
#[derive(Debug, Clone)]
pub struct InputNode {
    /// Natural horizontal extent of this node's cluster (e.g. card
    /// width for a single card, card + bar + card for a host that
    /// hosts one marriage). Walker positions the node's *center*; the
    /// width contributes the left and right contour points.
    pub width: f64,
    /// Indices (into the same `Vec<InputNode>`) of this node's
    /// children in declaration order.
    pub children: Vec<usize>,
}

/// Per-node result after [`run`]. The `x` value is the cluster's
/// center on the layout x-axis; the caller derives the cluster's
/// left edge by subtracting `width / 2`.
#[derive(Debug, Clone, Copy)]
pub struct LaidOut {
    /// Final x-coordinate of the cluster's center.
    pub x: f64,
}

/// Run Walker's algorithm over `nodes` rooted at the indices in
/// `roots`, separated by `sibling_gap`. Returns one [`LaidOut`] per
/// input node, indexed identically.
///
/// Multiple roots are positioned as if they were siblings under a
/// virtual super-root — they share the same collision-avoidance pass
/// and end up left-to-right in `roots` order.
pub fn run(nodes: &[InputNode], roots: &[usize], sibling_gap: f64) -> Vec<LaidOut> {
    let n = nodes.len();
    if n == 0 || roots.is_empty() {
        return Vec::new();
    }
    let mut state: Vec<State> = (0..n)
        .map(|i| State {
            parent: None,
            number: 0,
            width: nodes[i].width,
            children: nodes[i].children.clone(),
            prelim: 0.0,
            modifier: 0.0,
            thread: None,
            ancestor: i,
            change: 0.0,
            shift: 0.0,
            x: 0.0,
        })
        .collect();

    // Stitch parent pointers and sibling numbers in one pass.
    for (i, node) in nodes.iter().enumerate() {
        for (n_idx, &child) in node.children.iter().enumerate() {
            state[child].parent = Some(i);
            state[child].number = n_idx;
        }
    }

    // Set sibling numbers on the roots themselves so the algorithm
    // can treat them as siblings of one virtual super-root.
    for (n_idx, &root) in roots.iter().enumerate() {
        state[root].number = n_idx;
    }

    // First walk over every root (post-order).
    for &root in roots {
        first_walk(&mut state, root, sibling_gap);
    }

    // Pack roots left-to-right. Walker centers each subtree on its
    // root's prelim, but the subtree's bounding-box left can differ
    // from `root.prelim - root.width/2` when descendants extend
    // further out. First do a tentative second_walk to read off the
    // natural bounding box, then re-walk with the shift baked into
    // the initial modifier so it propagates to every descendant.
    let mut cursor = 0.0_f64;
    for (i, &root) in roots.iter().enumerate() {
        second_walk(&mut state, root, 0.0);
        let extent = subtree_extent(&state, root);
        let delta = cursor - extent.min_x;
        if delta != 0.0 {
            second_walk(&mut state, root, delta);
        }
        let final_extent = subtree_extent(&state, root);
        cursor = final_extent.max_x
            + if i + 1 < roots.len() {
                sibling_gap
            } else {
                0.0
            };
    }

    state.iter().map(|s| LaidOut { x: s.x }).collect()
}

#[derive(Debug, Clone)]
struct State {
    parent: Option<usize>,
    number: usize,
    width: f64,
    children: Vec<usize>,
    prelim: f64,
    modifier: f64,
    thread: Option<usize>,
    ancestor: usize,
    change: f64,
    shift: f64,
    x: f64,
}

fn first_walk(state: &mut [State], v: usize, sibling_gap: f64) {
    if state[v].children.is_empty() {
        // Leaf: prelim is its own offset from a left sibling, if any.
        if let Some(w) = left_sibling(state, v) {
            state[v].prelim = state[w].prelim + min_separation(state, v, w, sibling_gap);
        } else {
            state[v].prelim = 0.0;
        }
    } else {
        let mut default_ancestor = state[v].children[0];
        for i in 0..state[v].children.len() {
            let w = state[v].children[i];
            first_walk(state, w, sibling_gap);
            default_ancestor = apportion(state, w, default_ancestor, sibling_gap);
        }
        execute_shifts(state, v);
        let first = *state[v].children.first().unwrap();
        let last = *state[v].children.last().unwrap();
        let midpoint = (state[first].prelim + state[last].prelim) / 2.0;
        if let Some(w) = left_sibling(state, v) {
            state[v].prelim = state[w].prelim + min_separation(state, v, w, sibling_gap);
            state[v].modifier = state[v].prelim - midpoint;
        } else {
            state[v].prelim = midpoint;
        }
    }
}

fn second_walk(state: &mut [State], v: usize, m: f64) {
    state[v].x = state[v].prelim + m;
    let children = state[v].children.clone();
    let new_m = m + state[v].modifier;
    for c in children {
        second_walk(state, c, new_m);
    }
}

fn left_sibling(state: &[State], v: usize) -> Option<usize> {
    let parent = state[v].parent?;
    let n = state[v].number;
    if n == 0 {
        return None;
    }
    Some(state[parent].children[n - 1])
}

fn leftmost_sibling(state: &[State], v: usize) -> Option<usize> {
    let parent = state[v].parent?;
    if state[v].number == 0 {
        return None;
    }
    Some(state[parent].children[0])
}

/// Required separation between adjacent siblings' centers given their
/// half-widths and the requested gap.
fn min_separation(state: &[State], a: usize, b: usize, sibling_gap: f64) -> f64 {
    state[a].width / 2.0 + sibling_gap + state[b].width / 2.0
}

fn apportion(
    state: &mut [State],
    v: usize,
    mut default_ancestor: usize,
    sibling_gap: f64,
) -> usize {
    let Some(w) = left_sibling(state, v) else {
        return default_ancestor;
    };
    let Some(leftmost) = leftmost_sibling(state, v) else {
        return default_ancestor;
    };

    let mut vir = v;
    let mut vor = v;
    let mut vil = w;
    let mut vol = leftmost;

    let mut sir = state[vir].modifier;
    let mut sor = state[vor].modifier;
    let mut sil = state[vil].modifier;
    let mut sol = state[vol].modifier;

    while let (Some(next_right_vil), Some(next_left_vir)) =
        (next_right(state, vil), next_left(state, vir))
    {
        vil = next_right_vil;
        vir = next_left_vir;
        vol = next_left(state, vol).unwrap_or(vol);
        vor = next_right(state, vor).unwrap_or(vor);
        state[vor].ancestor = v;

        let shift = (state[vil].prelim + sil) - (state[vir].prelim + sir)
            + min_separation(state, vil, vir, sibling_gap);
        if shift > 0.0 {
            let ancestor = ancestor_for(state, vil, v, default_ancestor);
            move_subtree(state, ancestor, v, shift);
            sir += shift;
            sor += shift;
        }

        sil += state[vil].modifier;
        sir += state[vir].modifier;
        sol += state[vol].modifier;
        sor += state[vor].modifier;
    }

    if next_right(state, vil).is_some() && next_right(state, vor).is_none() {
        state[vor].thread = next_right(state, vil);
        state[vor].modifier += sil - sor;
    }
    if next_left(state, vir).is_some() && next_left(state, vol).is_none() {
        state[vol].thread = next_left(state, vir);
        state[vol].modifier += sir - sol;
        default_ancestor = v;
    }
    default_ancestor
}

fn next_left(state: &[State], v: usize) -> Option<usize> {
    if let Some(&c) = state[v].children.first() {
        Some(c)
    } else {
        state[v].thread
    }
}

fn next_right(state: &[State], v: usize) -> Option<usize> {
    if let Some(&c) = state[v].children.last() {
        Some(c)
    } else {
        state[v].thread
    }
}

fn move_subtree(state: &mut [State], wl: usize, wr: usize, shift: f64) {
    let subtrees = (state[wr].number as i64 - state[wl].number as i64) as f64;
    if subtrees == 0.0 {
        return;
    }
    state[wr].change -= shift / subtrees;
    state[wr].shift += shift;
    state[wl].change += shift / subtrees;
    state[wr].prelim += shift;
    state[wr].modifier += shift;
}

fn execute_shifts(state: &mut [State], v: usize) {
    let mut shift = 0.0_f64;
    let mut change = 0.0_f64;
    for i in (0..state[v].children.len()).rev() {
        let w = state[v].children[i];
        state[w].prelim += shift;
        state[w].modifier += shift;
        change += state[w].change;
        shift += state[w].shift + change;
    }
}

fn ancestor_for(state: &[State], vil: usize, v: usize, default_ancestor: usize) -> usize {
    let Some(parent) = state[v].parent else {
        return default_ancestor;
    };
    let vil_ancestor = state[vil].ancestor;
    if state[vil_ancestor].parent == Some(parent) {
        vil_ancestor
    } else {
        default_ancestor
    }
}

#[derive(Debug, Clone, Copy)]
struct Extent {
    min_x: f64,
    max_x: f64,
}

fn subtree_extent(state: &[State], root: usize) -> Extent {
    let mut min_x = state[root].x - state[root].width / 2.0;
    let mut max_x = state[root].x + state[root].width / 2.0;
    let mut stack = vec![root];
    while let Some(v) = stack.pop() {
        let left = state[v].x - state[v].width / 2.0;
        let right = state[v].x + state[v].width / 2.0;
        if left < min_x {
            min_x = left;
        }
        if right > max_x {
            max_x = right;
        }
        stack.extend(state[v].children.iter().copied());
    }
    Extent { min_x, max_x }
}
