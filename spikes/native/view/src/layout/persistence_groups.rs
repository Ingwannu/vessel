//! Serialized split identities must preserve independent resize routing.
use super::{GroupId, Layout, LayoutMode, PaneId, Position, SplitFraction, SplitNode};
use std::collections::BTreeMap;
fn nested() -> Layout {
    let mut layout = Layout::default();
    let side = layout
        .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
        .unwrap();
    let _below = layout
        .split(PaneId::MAIN, Position::Bottom, LayoutMode::SplitPanes)
        .unwrap();
    let _side_below = layout
        .split(side, Position::Bottom, LayoutMode::SplitPanes)
        .unwrap();
    layout
}
fn decode(layout: &Layout) -> Result<Layout, serde_json::Error> {
    let encoded = serde_json::to_string(layout)?;
    serde_json::from_str(&encoded)
}
#[test]
fn restored_ancestor_and_sibling_group_collisions_are_rejected() {
    let layout = nested();
    let mut ancestor = layout.clone();
    let SplitNode::Group { id, children, .. } = &mut ancestor.root else {
        panic!("expected root split");
    };
    let SplitNode::Group { id: child, .. } = children.first_mut().unwrap().as_mut() else {
        panic!("expected nested split");
    };
    *child = *id;
    assert!(decode(&ancestor).is_err());
    let mut siblings = layout;
    let SplitNode::Group { children, .. } = &mut siblings.root else {
        panic!("expected root split");
    };
    let SplitNode::Group { id: first, .. } = children.first().unwrap().as_ref() else {
        panic!("expected nested split");
    };
    let first = *first;
    let SplitNode::Group { id, .. } = children.last_mut().unwrap().as_mut() else {
        panic!("expected nested split");
    };
    *id = first;
    assert!(decode(&siblings).is_err());
}
#[test]
fn restored_group_cannot_collide_with_the_next_or_later_allocation() {
    for invalid in [5, u64::MAX] {
        let mut layout = nested();
        assert_eq!(layout.next_pane, 5);
        let SplitNode::Group { id, .. } = &mut layout.root else {
            panic!("expected root split");
        };
        *id = GroupId(invalid);
        assert!(decode(&layout).is_err());
    }
}
fn shares(node: &SplitNode, result: &mut BTreeMap<GroupId, SplitFraction>) {
    match node {
        SplitNode::Pane(_) => {}
        SplitNode::Group {
            id,
            fraction,
            children,
            ..
        } => {
            let _previous = result.insert(*id, *fraction);
            for child in children {
                shares(child, result);
            }
        }
    }
}
#[test]
fn valid_unique_nested_groups_roundtrip_and_resize_independently() {
    let mut layout = nested();
    layout.resize(GroupId(2), 0.25).unwrap();
    layout.resize(GroupId(3), 0.75).unwrap();
    let mut restored = decode(&layout).unwrap();
    assert_eq!(restored, layout);
    restored.resize(GroupId(4), 0.25).unwrap();
    let mut actual = BTreeMap::new();
    shares(&restored.root, &mut actual);
    assert_eq!(
        actual,
        BTreeMap::from([
            (GroupId(2), SplitFraction::clamped(0.25).unwrap()),
            (GroupId(3), SplitFraction::clamped(0.75).unwrap()),
            (GroupId(4), SplitFraction::clamped(0.25).unwrap()),
        ])
    );
}
