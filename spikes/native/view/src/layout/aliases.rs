//! Compatibility keys follow an acknowledged book edit without inferring cwd ownership.
use super::{HostAlias, Layout, LayoutError, LayoutKey};
use std::collections::BTreeMap;
/// Preserve legacy session layouts when only the book's display alias changes.
///
/// # Errors
/// Refuses destination collisions before moving any retained work.
pub fn rename_alias(
    book: &mut BTreeMap<LayoutKey, Layout>,
    from: &HostAlias,
    to: &HostAlias,
) -> Result<(), LayoutError> {
    if from == to {
        return Ok(());
    }
    let moves: Vec<_> = book
        .keys()
        .filter_map(|key| match key {
            LayoutKey::Session { machine, session } if machine == from => Some((
                key.clone(),
                LayoutKey::Session {
                    machine: to.clone(),
                    session: *session,
                },
            )),
            LayoutKey::Session { .. } | LayoutKey::Workspace { .. } => None,
        })
        .collect();
    if moves
        .iter()
        .any(|(_, destination)| book.contains_key(destination))
    {
        return Err(LayoutError::InvalidState);
    }
    for (source, destination) in moves {
        let layout = book.remove(&source).ok_or(LayoutError::InvalidState)?;
        let _previous = book.insert(destination, layout);
    }
    Ok(())
}
/// Forget drops clean presentation and retains modified work for recovery.
/// Retained alias-keyed metadata cannot authorize I/O: feature owners still need
/// the originally admitted node binding before opening or mutating a resource.
pub fn forget_alias(book: &mut BTreeMap<LayoutKey, Layout>, alias: &HostAlias) {
    book.retain(|key, layout| match key {
        LayoutKey::Session { machine, .. } => {
            machine != alias
                || layout
                    .tabs()
                    .any(|tab| tab.modified() == super::Modified::Dirty)
        }
        LayoutKey::Workspace { .. } => true,
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rename_preserves_layout_and_collision_is_atomic() {
        let from = HostAlias::parse("before").unwrap();
        let to = HostAlias::parse("after").unwrap();
        let session = vessel_proto::SessionId::random();
        let old = LayoutKey::Session {
            machine: from.clone(),
            session,
        };
        let new = LayoutKey::Session {
            machine: to.clone(),
            session,
        };
        let mut book = BTreeMap::from([(old.clone(), Layout::default())]);
        rename_alias(&mut book, &from, &to).unwrap();
        assert!(book.contains_key(&new));
        assert!(!book.contains_key(&old));
        let _previous = book.insert(old, Layout::default());
        let before = book.clone();
        assert_eq!(
            rename_alias(&mut book, &from, &to),
            Err(LayoutError::InvalidState)
        );
        assert_eq!(book, before);
        forget_alias(&mut book, &from);
        assert!(book.contains_key(&new));
        assert_eq!(book.len(), 1);
    }
}

#[cfg(test)]
mod protection_tests {
    use super::*;
    use crate::layout::{AgentRole, DraftId, LayoutMode, Modified, OpenIntent, Placement, Target};
    #[test]
    fn forgetting_host_removes_clean_layout_but_preserves_modified_draft_and_tab_identity() {
        let alias = HostAlias::parse("before").unwrap();
        let dirty_key = LayoutKey::Session {
            machine: alias.clone(),
            session: vessel_proto::SessionId::random(),
        };
        let clean_key = LayoutKey::Session {
            machine: alias.clone(),
            session: vessel_proto::SessionId::random(),
        };
        let mut layout = Layout::default();
        let target = Target::Draft(DraftId::random());
        let tab = layout
            .open(
                target.clone(),
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap();
        layout.set_modified(tab, Modified::Dirty).unwrap();
        let mut book = BTreeMap::from([
            (dirty_key.clone(), layout),
            (clean_key.clone(), Layout::default()),
        ]);
        forget_alias(&mut book, &alias);
        assert!(!book.contains_key(&clean_key));
        let Some(retained) = book.get(&dirty_key) else {
            panic!("host forget discarded the modified draft");
        };
        assert_eq!(retained.tab(tab).unwrap().target(), &target);
        assert_eq!(retained.tab(tab).unwrap().modified(), Modified::Dirty);
        let durable = super::super::RememberedLayout::new(dirty_key, retained);
        let decoded: super::super::RememberedLayout =
            serde_json::from_str(&serde_json::to_string(&durable).unwrap()).unwrap();
        assert_eq!(decoded.layout.tab(tab).unwrap().target(), &target);
        assert_eq!(decoded.layout.tab(tab).unwrap().modified(), Modified::Dirty);
    }
}
