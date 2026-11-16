//! Typed three-way deltas preserve stale windows' unrelated fields and map entries.

mod layouts;

use crate::{Kept, Prefs};
use std::collections::{BTreeMap, BTreeSet};
use vessel_app_view::prefs::{
    AgentDefaults, Appearance, Composer, Layout, MAX_ACTIONS, MAX_MACHINES, MachineDefaults,
    Notifications, Preferences, Refused, SidebarView,
};

fn field<T: PartialEq + Clone>(current: &mut T, before: &T, after: &T) {
    if before != after {
        *current = after.clone();
    }
}

fn map<K: Ord + Clone, V: PartialEq + Clone>(
    current: &mut BTreeMap<K, V>,
    before: &BTreeMap<K, V>,
    after: &BTreeMap<K, V>,
) {
    merge_map(current, before, after, |target, _before, after| {
        target.clone_from(after);
    });
}

fn merge_map<K: Ord + Clone, V: PartialEq + Clone>(
    current: &mut BTreeMap<K, V>,
    before: &BTreeMap<K, V>,
    after: &BTreeMap<K, V>,
    changed: fn(&mut V, Option<&V>, &V),
) {
    for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        if before.get(key) == after.get(key) {
            continue;
        }
        match after.get(key) {
            Some(value) => match current.get_mut(key) {
                Some(target) => changed(target, before.get(key), value),
                None => {
                    let _old = current.insert(key.clone(), value.clone());
                }
            },
            None => {
                let _old = current.remove(key);
            }
        }
    }
}

macro_rules! fields {
    ($current:expr,$before:expr,$after:expr; $($field:ident),+ $(,)?)=>{$(field(&mut $current.$field,&$before.$field,&$after.$field);)+};
}

pub(super) fn apply(current: &Prefs, before: &Prefs, after: &Prefs) -> Result<Prefs, Refused> {
    // Exhaustive patterns make a new persisted field demand an explicit merge
    // policy, instead of silently resetting it when a second window saves.
    let Prefs {
        machine: _,
        agent: _,
        folders: _,
        choices: _,
        manual_updates: _,
        last_label: _,
        update_notes: _,
        settings: _,
        layouts: _,
        kept: _,
    } = after;
    let mut next = current.clone();
    fields!(next,before,after;machine,agent,manual_updates,last_label,update_notes);
    map(&mut next.folders, &before.folders, &after.folders);
    merge_map(
        &mut next.choices,
        &before.choices,
        &after.choices,
        |current, before, after| {
            kept_agents(current, before.map(Vec::as_slice), after);
        },
    );
    preferences(&mut next.settings, &before.settings, &after.settings);
    layouts::apply(&mut next.layouts, &before.layouts, &after.layouts);
    let grew = |prior: usize, next: usize, max: usize| next > max && next > prior;
    if grew(
        current.settings.shortcuts.len(),
        next.settings.shortcuts.len(),
        MAX_ACTIONS,
    ) {
        return Err(Refused::Actions);
    }
    if grew(
        current.settings.defaults.len(),
        next.settings.defaults.len(),
        MAX_MACHINES,
    ) {
        return Err(Refused::Machines);
    }
    Ok(next)
}

fn kept_agents(current: &mut Vec<Kept>, before: Option<&[Kept]>, after: &[Kept]) {
    let before = before.unwrap_or_default();
    current.retain(|kept| {
        !before.iter().any(|old| old.agent == kept.agent)
            || after.iter().any(|new| new.agent == kept.agent)
    });
    for new in after {
        if before.iter().find(|old| old.agent == new.agent) == Some(new) {
            continue;
        }
        match current.iter_mut().find(|kept| kept.agent == new.agent) {
            Some(kept) => kept.choices.clone_from(&new.choices),
            None => current.push(new.clone()),
        }
    }
}

fn preferences(current: &mut Preferences, before: &Preferences, after: &Preferences) {
    let Preferences {
        language: _,
        appearance: _,
        layout: _,
        composer: _,
        shortcuts: _,
        notifications: _,
        sidebar: _,
        defaults: _,
        terminal_scrollback: _,
    } = after;
    fields!(current,before,after;language,terminal_scrollback);
    appearance(&mut current.appearance, before.appearance, after.appearance);
    layout(&mut current.layout, &before.layout, &after.layout);
    composer(&mut current.composer, before.composer, after.composer);
    notifications(
        &mut current.notifications,
        before.notifications,
        after.notifications,
    );
    sidebar(&mut current.sidebar, before.sidebar, after.sidebar);
    map(&mut current.shortcuts, &before.shortcuts, &after.shortcuts);
    merge_map(
        &mut current.defaults,
        &before.defaults,
        &after.defaults,
        machine_defaults,
    );
}

fn appearance(current: &mut Appearance, before: Appearance, after: Appearance) {
    let Appearance {
        theme: _,
        interface_size: _,
        code_size: _,
        content_width: _,
        conversation: _,
        animations: _,
    } = after;
    fields!(current,before,after;theme,interface_size,code_size,content_width,conversation,animations);
}

fn layout(current: &mut Layout, before: &Layout, after: &Layout) {
    let Layout {
        sidebar: _,
        inspector: _,
        sidebar_width: _,
        panel_width: _,
    } = after;
    fields!(current,before,after;sidebar,inspector,sidebar_width,panel_width);
}

fn composer(current: &mut Composer, before: Composer, after: Composer) {
    let Composer {
        send: _,
        delivery: _,
    } = after;
    fields!(current,before,after;send,delivery);
}

fn notifications(current: &mut Notifications, before: Notifications, after: Notifications) {
    let Notifications {
        alerts: _,
        title: _,
    } = after;
    fields!(current,before,after;alerts,title);
}

fn sidebar(current: &mut SidebarView, before: SidebarView, after: SidebarView) {
    let SidebarView {
        group: _,
        sort: _,
        ended: _,
    } = after;
    fields!(current,before,after;group,sort,ended);
}

fn machine_defaults(
    current: &mut MachineDefaults,
    before: Option<&MachineDefaults>,
    after: &MachineDefaults,
) {
    let MachineDefaults {
        agent: _,
        agents: _,
    } = after;
    // An absent old entry is its default record, not permission to overwrite
    // a concurrent window's independently created fields on the same host.
    let empty = MachineDefaults::default();
    let before = before.unwrap_or(&empty);
    field(&mut current.agent, &before.agent, &after.agent);
    merge_map(
        &mut current.agents,
        &before.agents,
        &after.agents,
        agent_defaults,
    );
}

fn agent_defaults(
    current: &mut AgentDefaults,
    before: Option<&AgentDefaults>,
    after: &AgentDefaults,
) {
    let AgentDefaults {
        profile: _,
        choices: _,
    } = after;
    let empty = AgentDefaults::default();
    let before = before.unwrap_or(&empty);
    // Choices is the view's validated atomic option bag. Cloning it preserves
    // its bounds; profile and choices remain independent form fields.
    fields!(current,before,after;profile,choices);
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_view::prefs::{Grouping, Theme};

    #[test]
    fn stale_nonsettings_choices_keep_other_agents_and_remove_only_the_named_agent() {
        let claude = Kept {
            agent: "claude-code".parse().unwrap(),
            choices: Vec::new(),
        };
        let codex = Kept {
            agent: "codex".parse().unwrap(),
            choices: Vec::new(),
        };
        let before = Prefs::default();
        let mut current = before.clone();
        let _old = current.choices.insert("host".into(), vec![claude.clone()]);
        let mut after = before.clone();
        let _old = after.choices.insert("host".into(), vec![codex.clone()]);
        let merged = apply(&current, &before, &after).unwrap();
        assert_eq!(
            &merged.choices["host"],
            &vec![claude.clone(), codex.clone()]
        );
        let mut old = before;
        let _prior = old.choices.insert("host".into(), vec![claude]);
        let mut removed = old.clone();
        removed.choices.get_mut("host").unwrap().clear();
        let next = apply(&merged, &old, &removed).unwrap();
        assert_eq!(&next.choices["host"], &vec![codex]);
    }

    #[test]
    fn concurrent_default_records_preserve_independent_host_profile_and_choice_fields() {
        use vessel_app_view::prefs::{AgentKind, Choices};
        use vessel_proto::{ChoiceValue, HarnessKind, NodeId, OptionId, ProfileName};
        let node = NodeId::random();
        let kind = AgentKind(HarnessKind::ClaudeCode);
        let profile: ProfileName = "work".parse().unwrap();
        let choices = Choices::try_from(BTreeMap::from([(
            OptionId::new("fast"),
            ChoiceValue::Toggle { on: true },
        )]))
        .unwrap();
        for existing in [false, true] {
            let mut before = Prefs::default();
            if existing {
                let _old = before
                    .settings
                    .defaults
                    .entry(node)
                    .or_default()
                    .agents
                    .insert(kind, AgentDefaults::default());
            }
            let mut current = before.clone();
            let machine = current.settings.defaults.entry(node).or_default();
            machine.agent = Some(HarnessKind::ClaudeCode);
            machine.agents.entry(kind).or_default().profile = Some(profile.clone());
            let mut after = before.clone();
            after
                .settings
                .defaults
                .entry(node)
                .or_default()
                .agents
                .entry(kind)
                .or_default()
                .choices = choices.clone();
            let next = apply(&current, &before, &after).unwrap();
            let machine = &next.settings.defaults[&node];
            let agent = &machine.agents[&kind];
            assert_eq!(machine.agent, Some(HarnessKind::ClaudeCode));
            assert_eq!(agent.profile.as_ref(), Some(&profile));
            assert_eq!(agent.choices, choices);
        }
    }

    #[test]
    fn stale_unrelated_settings_and_nonsettings_fields_survive() {
        let before = Prefs::default();
        let mut current = before.clone();
        current.settings.appearance.theme = Theme::Light;
        let _old = current.folders.insert("other".into(), "/other".into());
        let mut after = before.clone();
        after.settings.sidebar.group = Grouping::Status;
        after.manual_updates = true;
        let next = apply(&current, &before, &after).unwrap();
        assert_eq!(next.settings.appearance.theme, Theme::Light);
        assert_eq!(next.settings.sidebar.group, Grouping::Status);
        assert_eq!(next.folders, current.folders);
        assert!(next.manual_updates);
    }

    #[test]
    fn map_removal_is_a_delta_and_same_field_last_edit_wins() {
        let mut before = Prefs::default();
        let _old = before.folders.insert("remove".into(), "/remove".into());
        let mut current = before.clone();
        let _old = current.folders.insert("keep".into(), "/keep".into());
        current.settings.appearance.theme = Theme::Light;
        let mut after = before.clone();
        after.folders.clear();
        after.settings.appearance.theme = Theme::Dark;
        let next = apply(&current, &before, &after).unwrap();
        assert!(!next.folders.contains_key("remove"));
        assert!(next.folders.contains_key("keep"));
        assert_eq!(next.settings.appearance.theme, Theme::Dark);
    }
}
