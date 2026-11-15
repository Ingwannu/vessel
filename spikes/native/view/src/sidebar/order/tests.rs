use super::*;
#[test]
fn stored_order_only_reorders_its_known_slots_until_discovery_reconciliation() {
    assert_eq!(
        apply(&['n', 'a', 'x', 'b'], &['b', 'a']),
        vec![&'n', &'b', &'x', &'a']
    );
    assert_eq!(apply(&['a', 'b'], &['s', 'b', 'b', 'a']), vec![&'b', &'a']);
    assert_eq!(apply(&['a', 'b'], &['x']), vec![&'a', &'b']);
}
#[test]
fn discovery_appends_projects_but_prepends_workspaces_and_never_prunes_saved_keys() {
    assert_eq!(
        reconcile(&['b', 'a', 'x'], &['n', 'a', 'b'], Missing::Append),
        Some(vec!['b', 'a', 'x', 'n'])
    );
    assert_eq!(
        reconcile(&['b', 'a', 'x'], &['n', 'a', 'b'], Missing::Prepend),
        Some(vec!['n', 'b', 'a', 'x'])
    );
    assert_eq!(
        reconcile(&['b', 'a', 'x'], &['a', 'b'], Missing::Append),
        None
    );
}
#[test]
fn visible_reorder_preserves_remainder_and_unchanged_input_needs_no_write() {
    assert_eq!(
        reordered(&['a', 'x', 'b', 'y'], &['b', 'a']),
        Some(vec!['b', 'a', 'x', 'y'])
    );
    assert_eq!(reordered(&['a', 'b', 'c', 'd'], &['a', 'b', 'c']), None);
    assert_eq!(
        reordered(&['a', 'b'], &['a', 'c']),
        Some(vec!['a', 'c', 'b'])
    );
    assert_eq!(reordered(&['x', 'y'], &[]), None);
}
#[test]
fn drop_intent_moves_only_the_captured_visible_group_and_refuses_stale_keys() {
    assert_eq!(moved(&[1, 2, 3], &3, &1, Side::Before), Some(vec![3, 1, 2]));
    assert_eq!(moved(&[1, 2, 3], &1, &3, Side::After), Some(vec![2, 3, 1]));
    assert_eq!(moved(&[1, 2, 3], &1, &1, Side::After), None);
    assert_eq!(moved(&[1, 2, 3], &9, &1, Side::Before), None);
    assert_eq!(moved(&[1, 2, 3], &1, &2, Side::Before), None);
}
