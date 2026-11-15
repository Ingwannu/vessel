//! The one binding table for matching, hints, help and rebinding. Reference
//! definitions retain Paseo's persisted ids and ordering; native extensions and
//! explicitly unbound non-Mac pane actions are appended with named origins.

mod actions;
mod agent_input;
mod extensions;
mod general;
mod layout;
mod tabs_panes;
mod workspaces;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::context::{Admission, CommandCenterState, FocusScope, HostKind, ShortcutContext, When};
use super::definitions::{Definition, build_effective};
use super::engine::{Consume, EffectiveBinding, Repeat, ShortcutIndex};
use super::keys::{Binding, Chord, Platform};
use super::sequence::{KeySequence, SequenceError};
use crate::prefs::{ActionId, Bindings, InvalidActionId};
pub use actions::Action;

/// Number of shipped reference binding variants, excluding named native additions.
pub const PASEO_BINDING_COUNT: usize = 76;

/// Help sections keep the reference's order and never depend on enum discriminants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutGroup {
    /// App-wide command center, Settings and appearance.
    General,
    /// Projects and workspaces.
    Navigation,
    /// Tabs and split panes.
    TabsPanes,
    /// Sidebars and focus layout.
    Panels,
    /// Composer and agent input.
    Session,
}

impl ShortcutGroup {
    /// Reference help and Settings section order.
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::Navigation,
        Self::TabsPanes,
        Self::Panels,
        Self::Session,
    ];
}

/// Compatibility projection for feature callers with no editable-focus query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutScope {
    /// An open command center owns scoped keys.
    CommandCenter,
    /// An actual terminal surface owns input.
    Terminal,
    /// Another non-editing surface.
    Elsewhere,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    Reference,
    Unassigned,
    Extension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpVisibility {
    Visible,
    Hidden,
}

#[derive(Debug)]
struct Spec {
    order: usize,
    id: &'static str,
    action: Action,
    combo: &'static str,
    when: When,
    repeat: Repeat,
    prevent_default: Consume,
    stop_propagation: Consume,
    help: HelpVisibility,
    origin: Origin,
}

macro_rules! binding {
    ($order:literal, $id:literal, $action:ident, $combo:literal, $when:expr, $repeat:expr, $prevent:expr, $stop:expr, $help:ident) => {
        Spec {
            order: $order,
            id: $id,
            action: Action::$action,
            combo: $combo,
            when: $when,
            repeat: $repeat,
            prevent_default: $prevent,
            stop_propagation: $stop,
            help: HelpVisibility::$help,
            origin: Origin::Reference,
        }
    };
}

macro_rules! extension {
    ($order:literal, $id:literal, $action:ident, $combo:literal, $when:expr, $origin:ident) => {
        Spec {
            order: $order,
            id: $id,
            action: Action::$action,
            combo: $combo,
            when: $when,
            repeat: Repeat::Allow,
            prevent_default: Consume::Yes,
            stop_propagation: Consume::Yes,
            help: HelpVisibility::Visible,
            origin: Origin::$origin,
        }
    };
}

// Child modules use these macros without exporting an application-wide macro.
use {binding, extension};

/// A static definition failure is typed and contains no preference-file contents.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// Every reference variant must be present before matching is enabled.
    #[error("shortcut reference binding table is incomplete")]
    IncompleteReference,
    /// Stable persistence ids cannot select two different definitions.
    #[error("duplicate shortcut definition identity")]
    DuplicateId,
    /// A source table id violated the preference contract.
    #[error("invalid shortcut definition identity")]
    Id(#[from] InvalidActionId),
    /// A source table key violated the shared grammar.
    #[error("invalid shortcut definition keys")]
    Keys(#[from] SequenceError),
}

fn specs() -> &'static [&'static Spec] {
    static SPECS: OnceLock<Vec<&'static Spec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        let sources = [
            general::BINDINGS,
            workspaces::BINDINGS,
            tabs_panes::BINDINGS,
            layout::BINDINGS,
            agent_input::BINDINGS,
            extensions::BINDINGS,
        ];
        let mut specs: Vec<_> = sources.into_iter().flatten().collect();
        specs.sort_by_key(|spec| spec.order);
        specs
    })
}

/// Parse compiled data once; every consumer uses these same typed definitions.
///
/// # Errors
/// A broken compiled definition is refused before any key may execute it.
pub fn definitions() -> Result<&'static [Definition<Action>], RegistryError> {
    static DEFINITIONS: OnceLock<Result<Vec<Definition<Action>>, RegistryError>> = OnceLock::new();
    DEFINITIONS
        .get_or_init(build_definitions)
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

fn build_definitions() -> Result<Vec<Definition<Action>>, RegistryError> {
    if specs()
        .iter()
        .filter(|spec| spec.origin == Origin::Reference)
        .count()
        != PASEO_BINDING_COUNT
    {
        return Err(RegistryError::IncompleteReference);
    }
    let mut seen = BTreeSet::new();
    let mut definitions = Vec::new();
    for spec in specs() {
        let id: ActionId = spec.id.parse()?;
        if !seen.insert(id.clone()) {
            return Err(RegistryError::DuplicateId);
        }
        definitions.push(Definition {
            id,
            action: spec.action,
            default_keys: if spec.combo.is_empty() {
                None
            } else {
                Some(KeySequence::parse(spec.combo)?)
            },
            payload: spec.action.payload(),
            when: spec.when,
            repeat: spec.repeat,
            prevent_default: spec.prevent_default,
            stop_propagation: spec.stop_propagation,
        });
    }
    Ok(definitions)
}

/// The current platform's first visible binding is the Settings/help row owner.
#[must_use]
pub fn binding_id(action: Action, context: ShortcutContext) -> Option<ActionId> {
    specs()
        .iter()
        .find(|spec| {
            spec.action == action
                && spec.help == HelpVisibility::Visible
                && spec.when.on_platform(context)
        })
        .and_then(|spec| spec.id.parse().ok())
}

/// Current-host rows, including explicitly unbound actions. Hidden aliases still
/// participate in matching and conflicts, but do not duplicate Settings rows.
///
/// # Errors
/// Refuses the whole table if a compiled definition is invalid.
pub fn visible_definitions(
    context: ShortcutContext,
) -> Result<Vec<&'static Definition<Action>>, RegistryError> {
    let definitions = definitions()?;
    Ok(Action::ALL
        .into_iter()
        .filter_map(|action| {
            let id = binding_id(action, context)?;
            definitions.iter().find(|definition| definition.id == id)
        })
        .collect())
}

/// Migrate historical action-level keys in memory before their next write. Unknown
/// ids and inactive-platform overrides survive; canonical ids always win.
#[must_use]
pub fn migrated_overrides(
    overrides: &BTreeMap<ActionId, Bindings>,
    context: ShortcutContext,
) -> BTreeMap<ActionId, Bindings> {
    let mut migrated = overrides.clone();
    let mut seen = BTreeSet::new();
    for spec in specs()
        .iter()
        .filter(|spec| spec.when.on_platform(context) && spec.help == HelpVisibility::Visible)
    {
        if !seen.insert(spec.action) {
            continue;
        }
        let Some(legacy) = spec.action.action_id() else {
            continue;
        };
        let Ok(id) = spec.id.parse::<ActionId>() else {
            continue;
        };
        if legacy == id {
            continue;
        }
        if let Some(keys) = overrides.get(&legacy) {
            drop(migrated.remove(&legacy));
            let _existing = migrated.entry(id).or_insert_with(|| keys.clone());
        }
    }
    migrated
}

/// The single effective snapshot used by matching, hints, help and conflicts.
///
/// # Errors
/// Refuses an invalid compiled registry instead of dispatching partial bindings.
pub fn effective_bindings(
    overrides: &BTreeMap<ActionId, Bindings>,
    context: ShortcutContext,
) -> Result<Vec<EffectiveBinding<Action>>, RegistryError> {
    let overrides = migrated_overrides(overrides, context);
    Ok(build_effective(definitions()?, &overrides)
        .bindings
        .into_iter()
        .filter(|binding| binding.when.on_platform(context))
        .collect())
}

/// The actual primary help/sidebar binding, including a user chord override.
/// None means explicitly unassigned or unavailable on this platform.
///
/// # Errors
/// Refuses invalid compiled registry data before displaying a misleading hint.
pub fn primary_keys(
    action: Action,
    overrides: &BTreeMap<ActionId, Bindings>,
    context: ShortcutContext,
) -> Result<Option<KeySequence>, RegistryError> {
    let Some(id) = binding_id(action, context) else {
        return Ok(None);
    };
    Ok(effective_bindings(overrides, context)?
        .into_iter()
        .find(|binding| binding.id == id)
        .map(|binding| binding.keys))
}

/// Neutral context for hints and compatibility consumers; focus predicates still
/// apply at dispatch time rather than hiding shortcuts from help.
#[must_use]
pub const fn default_context(platform: Platform) -> ShortcutContext {
    ShortcutContext {
        platform,
        host: if cfg!(target_arch = "wasm32") {
            HostKind::Web
        } else {
            HostKind::Desktop
        },
        focus: FocusScope::Other,
        command_center: CommandCenterState::Closed,
        admission: Admission::Available,
    }
}

pub(super) fn default_combos(action: Action, platform: Platform) -> Vec<String> {
    let context = default_context(platform);
    specs()
        .iter()
        .filter(|spec| {
            spec.action == action && spec.when.on_platform(context) && !spec.combo.is_empty()
        })
        .filter_map(|spec| KeySequence::parse(spec.combo).ok())
        .map(|keys| keys.text())
        .collect()
}

pub(super) fn enabled(action: Action, scope: ShortcutScope, platform: Platform) -> bool {
    let mut context = default_context(platform);
    match scope {
        ShortcutScope::CommandCenter => {
            context.focus = FocusScope::CommandCenter;
            context.command_center = CommandCenterState::Open;
        }
        ShortcutScope::Terminal => context.focus = FocusScope::Terminal,
        ShortcutScope::Elsewhere => {}
    }
    specs()
        .iter()
        .any(|spec| spec.action == action && spec.when.matches(context))
}

/// Compatibility hints derive from the same effective entries, never a second
/// default table. New multi-step screens use `effective_bindings` directly.
#[must_use]
pub fn bindings_of(action: Action, overrides: &BTreeMap<ActionId, Bindings>) -> Vec<Binding> {
    bindings_on(action, overrides, Platform::native())
}

pub(super) fn bindings_on(
    action: Action,
    overrides: &BTreeMap<ActionId, Bindings>,
    platform: Platform,
) -> Vec<Binding> {
    effective_bindings(overrides, default_context(platform))
        .into_iter()
        .flatten()
        .filter(|binding| binding.action == action)
        .filter_map(|binding| Binding::parse(&binding.keys.text()))
        .collect()
}

/// Whether any current-platform binding or historical alias was changed.
#[must_use]
pub fn modified(action: Action, overrides: &BTreeMap<ActionId, Bindings>) -> bool {
    overrides.keys().any(|id| {
        id.as_str() == action.id()
            || specs()
                .iter()
                .any(|spec| spec.action == action && spec.id == id.as_str())
    })
}

/// The default desktop workspace jump uses its primary modifier, not fixed Alt.
#[must_use]
pub fn jump_of(chord: &Chord) -> Option<u8> {
    if !chord.primary || chord.ctrl || chord.shift || chord.meta || chord.alt {
        return None;
    }
    ShortcutIndex::from_key(&chord.key).map(ShortcutIndex::get)
}

/// The actual default index-template keys, before a user override.
#[must_use]
pub fn jump_shown() -> Chord {
    Chord {
        primary: true,
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
        key: "1–9".to_owned(),
    }
}

#[cfg(test)]
mod tests;
