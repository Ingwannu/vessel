//! Compatibility chord entry point for retained callers and regression fixtures.
//! All matching delegates to the same context-aware Engine used by the app;
//! there is no separate legacy resolution order or second binding registry.

use super::engine::{Engine, KeyRepeat, Resolution};
use super::{Action, Chord, Platform, default_context, effective_bindings};
use crate::prefs::{ActionId, Bindings};
use std::collections::BTreeMap;
use std::time::Duration;

/// The reference deadline, shared with the active engine.
pub use super::engine::CHORD_TIMEOUT_MS as SEQUENCE_MS;

/// An opaque engine snapshot preserves the exact candidate set between steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending(Engine);

/// What a normalized chord did through the common engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matched {
    /// A feature action, with its optional validated index unwrapped for legacy callers.
    Run {
        /// Original typed action.
        action: Action,
        /// One-based workspace/tab index.
        index: Option<u8>,
    },
    /// Keep this snapshot until another action key arrives.
    Wait(Pending),
    /// Leave the key with the focused control.
    None,
}

/// Resolve a normalized fixture chord without bypassing the real engine.
#[must_use]
pub fn matched(
    chord: &Chord,
    pending: Option<&Pending>,
    overrides: &BTreeMap<ActionId, Bindings>,
    now: u64,
    platform: Platform,
) -> Matched {
    let context = default_context(platform);
    let Ok(bindings) = effective_bindings(overrides, context) else {
        return Matched::None;
    };
    let mut engine = pending.map(|pending| pending.0.clone()).unwrap_or_default();
    match engine.resolve(
        chord,
        KeyRepeat::Initial,
        Duration::from_millis(now),
        context,
        &bindings,
    ) {
        Resolution::Pass => Matched::None,
        Resolution::Waiting => Matched::Wait(Pending(engine)),
        Resolution::Run {
            action,
            index,
            prevent_default: _,
            stop_propagation: _,
        } => Matched::Run {
            action,
            index: index.map(super::engine::ShortcutIndex::get),
        },
    }
}
