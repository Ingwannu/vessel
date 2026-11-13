//! Sessions as a tree (DESIGN.md §2.4; ADR 0013; the page's
//! `data/tree.ts`): a session an agent started nests under the session
//! whose agent started it, on whichever machine either runs.

use std::collections::HashMap;

use vessel_proto::{Creator, SessionId};

use super::{Card, Fleet};

/// A card with its place in the tree.
#[derive(Debug, Clone, Copy)]
pub struct Row<'a> {
    /// The session.
    pub card: &'a Card,
    /// How deep it nests (0 for a session a person started).
    pub depth: usize,
    /// The session whose agent started it, when that one is listed (it may
    /// run on another machine: its card says which).
    pub parent: Option<&'a Card>,
}

impl Fleet {
    /// Every session of the machine `scope` names (of every machine for
    /// `None`) as tree rows: roots (started by a person) newest first, each
    /// followed by its agent-started children, depth first. A child whose
    /// parent is not listed is a root.
    #[must_use]
    pub fn tree(&self, scope: Option<&str>) -> Vec<Row<'_>> {
        let all: Vec<&Card> = self
            .machines
            .iter()
            .filter(|machine| scope.is_none_or(|name| machine.name == name))
            .flat_map(|machine| machine.sessions.iter())
            .collect();
        let listed: HashMap<SessionId, &Card> =
            all.iter().map(|card| (card.session, *card)).collect();
        let mut children: HashMap<SessionId, Vec<&Card>> = HashMap::new();
        let mut roots: Vec<&Card> = Vec::new();
        for card in &all {
            match &card.detail.created_by {
                Creator::Agent { session, .. } if listed.contains_key(session) => {
                    children.entry(*session).or_default().push(card);
                }
                Creator::Owner | Creator::Device { .. } | Creator::Agent { .. } => {
                    roots.push(card);
                }
            }
        }
        let newest = |one: &&Card, two: &&Card| two.detail.created_at.cmp(&one.detail.created_at);
        roots.sort_by(newest);
        for list in children.values_mut() {
            list.sort_by(newest);
        }
        // Depth first without recursion: each card's children go on the
        // stack in reverse, so the newest comes off first. Every card has
        // one creator, so nothing reached from a root is reached twice.
        let mut rows = Vec::with_capacity(all.len());
        let mut stack: Vec<Row<'_>> = roots
            .iter()
            .rev()
            .map(|card| Row {
                card,
                depth: 0,
                parent: None,
            })
            .collect();
        while let Some(row) = stack.pop() {
            if let Some(list) = children.get(&row.card.session) {
                stack.extend(list.iter().rev().map(|child| Row {
                    card: child,
                    depth: row.depth.saturating_add(1),
                    parent: Some(row.card),
                }));
            }
            rows.push(row);
        }
        rows
    }
}
