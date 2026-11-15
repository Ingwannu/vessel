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
/// Forget removes presentation only; it never closes resources on the node.
pub fn forget_alias(book: &mut BTreeMap<LayoutKey, Layout>, alias: &HostAlias) {
    book.retain(|key, _| match key {
        LayoutKey::Session { machine, .. } => machine != alias,
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
