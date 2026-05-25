//! Walker's-algorithm unit tests with hand-fabricated trees.
//!
//! The example corpus does not naturally
//! exercise sibling-subtree overlap (the case Walker's
//! collision-avoidance machinery exists for). These tests cover the
//! algorithm's contract independent of the canonical-pattern adapter.

use kul_layout::walker::{InputNode, run};

fn leaf(width: f64) -> InputNode {
    InputNode {
        width,
        children: Vec::new(),
    }
}

fn branch(width: f64, children: Vec<usize>) -> InputNode {
    InputNode { width, children }
}

/// Single root, no children: returned x is 0 (root center) and the
/// extent matches the node's width.
#[test]
fn single_root_centers_at_zero() {
    let nodes = vec![leaf(100.0)];
    let positions = run(&nodes, &[0], 10.0);
    // Packed against cursor=0 with left-aligned bounding box.
    // root.x = 0 + width/2 = 50.
    assert_eq!(positions[0].x, 50.0);
}

/// Two siblings under one parent: parent centers above the midpoint
/// of its children.
#[test]
fn parent_centers_above_two_children() {
    // Tree:
    //     0 (parent, width 30)
    //    / \
    //   1   2 (leaves, width 40 each)
    let nodes = vec![branch(30.0, vec![1, 2]), leaf(40.0), leaf(40.0)];
    let positions = run(&nodes, &[0], 20.0);
    // Children sit at prelim 0 and 60 (40/2 + 20 + 40/2 = 60).
    // Parent prelim = midpoint = 30.
    // Subtree extent: left = 0 - 40/2 = -20, right = 60 + 40/2 = 80.
    // Packed against cursor=0: delta = 0 - (-20) = 20.
    // Final: parent.x = 50, child1.x = 20, child2.x = 80.
    assert_eq!(positions[0].x, 50.0);
    assert_eq!(positions[1].x, 20.0);
    assert_eq!(positions[2].x, 80.0);
    // Parent center sits exactly between the two children's centers.
    let mid = (positions[1].x + positions[2].x) / 2.0;
    assert!((positions[0].x - mid).abs() < 1e-9);
}

/// Sibling subtrees that would overlap without collision avoidance.
/// Walker should shift the right subtree out so the deepest contour
/// points stay separated by at least `sibling_gap`.
#[test]
fn sibling_subtrees_avoid_collision() {
    // Tree:
    //         0 (root, width 30)
    //        / \
    //       1   2 (intermediates, width 30 each)
    //      /|   |\
    //     3 4   5 6 (deep leaves, width 40 each)
    //
    // Without collision avoidance, left subtree's right-most leaf (4)
    // would overlap right subtree's left-most leaf (5). Walker's
    // apportion + threads should detect this and shift subtree 2 out.
    let nodes = vec![
        branch(30.0, vec![1, 2]), // 0: root
        branch(30.0, vec![3, 4]), // 1: left intermediate
        branch(30.0, vec![5, 6]), // 2: right intermediate
        leaf(40.0),               // 3
        leaf(40.0),               // 4
        leaf(40.0),               // 5
        leaf(40.0),               // 6
    ];
    let positions = run(&nodes, &[0], 20.0);
    // Deepest leaves should be separated by at least sibling_gap.
    let leaf_gap = positions[5].x - positions[4].x;
    assert!(
        leaf_gap >= 60.0 - 1e-9,
        "expected leaves 4 and 5 to be at least width+gap apart, got {leaf_gap}"
    );
    // Root centers above (intermediate1.x + intermediate2.x) / 2.
    let mid = (positions[1].x + positions[2].x) / 2.0;
    assert!((positions[0].x - mid).abs() < 1e-9);
}

/// Degenerate single-child paths: each node's center sits directly
/// above its sole child.
#[test]
fn single_child_path_aligns_centers() {
    // 0 → 1 → 2 → 3, each width 40
    let nodes = vec![
        branch(40.0, vec![1]),
        branch(40.0, vec![2]),
        branch(40.0, vec![3]),
        leaf(40.0),
    ];
    let positions = run(&nodes, &[0], 16.0);
    for w in positions.windows(2) {
        assert!(
            (w[0].x - w[1].x).abs() < 1e-9,
            "expected single-child path to align centers, got {w:?}"
        );
    }
}

/// Empty input returns empty output (algorithm precondition).
#[test]
fn empty_tree_returns_empty_positions() {
    let positions = run(&[], &[], 10.0);
    assert!(positions.is_empty());
}

/// Multiple roots pack left-to-right separated by sibling_gap.
#[test]
fn multiple_roots_pack_left_to_right() {
    // Two independent single-card components.
    let nodes = vec![leaf(100.0), leaf(60.0)];
    let positions = run(&nodes, &[0, 1], 24.0);
    // Root 0 occupies [0, 100], so its center is 50.
    // Root 1 starts at 100 + 24 = 124, occupies [124, 184], center 154.
    assert_eq!(positions[0].x, 50.0);
    assert_eq!(positions[1].x, 154.0);
}
