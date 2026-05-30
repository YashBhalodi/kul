//! Walker's-algorithm unit tests with hand-fabricated trees. The
//! example corpus doesn't exercise sibling-subtree overlap.

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

#[test]
fn single_root_centers_at_zero() {
    let nodes = vec![leaf(100.0)];
    let positions = run(&nodes, &[0], 10.0);
    assert_eq!(positions[0].x, 50.0);
}

#[test]
fn parent_centers_above_two_children() {
    //     0 (parent, w 30)
    //    / \
    //   1   2 (leaves, w 40)
    let nodes = vec![branch(30.0, vec![1, 2]), leaf(40.0), leaf(40.0)];
    let positions = run(&nodes, &[0], 20.0);
    assert_eq!(positions[0].x, 50.0);
    assert_eq!(positions[1].x, 20.0);
    assert_eq!(positions[2].x, 80.0);
    let mid = (positions[1].x + positions[2].x) / 2.0;
    assert!((positions[0].x - mid).abs() < 1e-9);
}

/// Sibling subtrees that would overlap without collision avoidance.
#[test]
fn sibling_subtrees_avoid_collision() {
    //         0 (w 30)
    //        / \
    //       1   2 (w 30 each)
    //      /|   |\
    //     3 4   5 6 (w 40 each)
    let nodes = vec![
        branch(30.0, vec![1, 2]),
        branch(30.0, vec![3, 4]),
        branch(30.0, vec![5, 6]),
        leaf(40.0),
        leaf(40.0),
        leaf(40.0),
        leaf(40.0),
    ];
    let positions = run(&nodes, &[0], 20.0);
    let leaf_gap = positions[5].x - positions[4].x;
    assert!(
        leaf_gap >= 60.0 - 1e-9,
        "expected leaves 4 and 5 to be at least width+gap apart, got {leaf_gap}"
    );
    let mid = (positions[1].x + positions[2].x) / 2.0;
    assert!((positions[0].x - mid).abs() < 1e-9);
}

/// Degenerate single-child paths: each center sits directly above its
/// sole child.
#[test]
fn single_child_path_aligns_centers() {
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

#[test]
fn empty_tree_returns_empty_positions() {
    let positions = run(&[], &[], 10.0);
    assert!(positions.is_empty());
}

#[test]
fn multiple_roots_pack_left_to_right() {
    let nodes = vec![leaf(100.0), leaf(60.0)];
    let positions = run(&nodes, &[0, 1], 24.0);
    assert_eq!(positions[0].x, 50.0);
    assert_eq!(positions[1].x, 154.0);
}
