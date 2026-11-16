//! Project the shared catalog into the existing list without fabricated wire commands.

use std::convert::Infallible;
use vessel_app_view::composer::slash::{Chosen, SlashRange, drawn};
use vessel_app_view::composer_shell::catalog::{self, Entry, Inline, Position, Source, Target};
use vessel_app_view::composer_shell::commands::ClientCommand;
use vessel_proto::AgentCommand;
use vessel_proto::composer_commands::CommandKind;

#[derive(Debug, Clone, Copy)]
pub(super) enum Choice<'a> {
    Client(ClientCommand),
    Provider(&'a AgentCommand),
}
impl<'a> Choice<'a> {
    pub(super) fn name(self) -> &'a str {
        match self {
            Self::Client(command) => command.name(),
            Self::Provider(command) => command.name.as_str(),
        }
    }
    pub(super) const fn client(self) -> Option<ClientCommand> {
        match self {
            Self::Client(command) => Some(command),
            Self::Provider(_) => None,
        }
    }
}

pub(super) struct Menu<'a> {
    pub(super) range: SlashRange,
    pub(super) rows: Vec<Choice<'a>>,
    pub(super) chosen: Option<usize>,
}
impl<'a> Menu<'a> {
    /// The shared interceptor queries this same predicate before native dispatch.
    pub(super) fn takes_key(
        &self,
        key: vessel_app_kit::field::composer_hooks::ComposerKey,
    ) -> bool {
        use vessel_app_kit::field::composer_hooks::ComposerKey;
        !self.rows.is_empty()
            && match key {
                ComposerKey::Escape => self.range.start == 0,
                ComposerKey::Up | ComposerKey::Down | ComposerKey::Enter | ComposerKey::Tab => true,
            }
    }
    pub(super) fn of(
        text: &str,
        caret: usize,
        target: Target,
        commands: Option<&'a [AgentCommand]>,
        chosen: &Chosen,
    ) -> Option<Self> {
        let active = catalog::active(text, caret)?;
        // Only canonical provider metadata enables inline completion. Older and
        // future/unknown peer kinds retain the root-only behavior.
        let providers: Vec<Entry<'a, Infallible>> = commands
            .unwrap_or_default()
            .iter()
            .map(|command| {
                Entry::provider(
                    command,
                    match command.kind {
                        Some(CommandKind::Skill) => Inline::Skill,
                        Some(CommandKind::Command | CommandKind::Unknown) | None => {
                            Inline::RootOnly
                        }
                    },
                )
            })
            .collect();
        // Inline matching starts from provider entries directly in the reference;
        // a provider skill may share a root-only built-in/plugin name.
        let entries = if active.position == Position::Inline {
            providers
        } else {
            catalog::merge(target, [], providers)
        };
        let rows = drawn(catalog::ranked(
            &entries,
            &active.range.query,
            active.position,
        ))
        .into_iter()
        .map(|entry| match &entry.source {
            Source::Client(command) => Choice::Client(*command),
            Source::Provider { command, .. } => Choice::Provider(command),
            Source::Plugin(uninhabited) => match **uninhabited {},
        })
        .collect::<Vec<_>>();
        let selected = chosen.clone().follow(&active.range.query, rows.len());
        Some(Self {
            range: active.range,
            rows,
            chosen: selected,
        })
    }
    pub(super) fn taken(&self, chosen: &Chosen) -> Option<Choice<'a>> {
        let mut chosen = chosen.clone();
        let _row = chosen.follow(&self.range.query, self.rows.len());
        chosen
            .taken(self.rows.len())
            .and_then(|index| self.rows.get(index).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_typed_rows_and_aliases_rank_canonical_names() {
        let choice = Chosen::default();
        let menu = Menu::of("/q", 2, Target::Session, None, &choice).unwrap();
        assert_eq!(menu.rows.len(), 1);
        let row = menu.taken(&choice).unwrap();
        assert_eq!(row.name(), "exit");
        assert_eq!(row.client(), Some(ClientCommand::Exit));
        assert!(
            Menu::of("/", 1, Target::Draft, None, &choice)
                .unwrap()
                .rows
                .is_empty()
        );
    }

    #[test]
    fn inline_menu_keeps_escape_for_the_field_but_owns_tab_and_enter() {
        use vessel_app_kit::field::composer_hooks::ComposerKey;
        let commands = [AgentCommand {
            name: "review".parse().unwrap(),
            description: String::new(),
            hint: None,
            kind: Some(CommandKind::Skill),
        }];
        let menu = Menu::of(
            "use /rev",
            8,
            Target::Session,
            Some(&commands),
            &Chosen::default(),
        )
        .unwrap();
        assert!(!menu.takes_key(ComposerKey::Escape));
        assert!(menu.takes_key(ComposerKey::Tab));
        assert!(menu.takes_key(ComposerKey::Enter));
        assert!(menu.takes_key(ComposerKey::Up));
        assert!(menu.takes_key(ComposerKey::Down));
    }

    #[test]
    fn old_peer_provider_names_do_not_become_inline_skills_or_shadow_builtins() {
        let commands = [AgentCommand {
            name: "q".parse().unwrap(),
            description: String::new(),
            hint: None,
            kind: None,
        }];
        let choice = Chosen::default();
        assert!(
            Menu::of("use /", 5, Target::Session, Some(&commands), &choice)
                .unwrap()
                .rows
                .is_empty()
        );
        assert!(
            Menu::of("/q", 2, Target::Draft, Some(&commands), &choice)
                .unwrap()
                .rows
                .is_empty()
        );
        assert_eq!(
            Menu::of("/q", 2, Target::Session, Some(&commands), &choice)
                .unwrap()
                .taken(&choice)
                .unwrap()
                .client(),
            Some(ClientCommand::Exit)
        );
    }

    #[test]
    fn explicit_skill_can_complete_inline_even_when_its_name_is_reserved_at_root() {
        let choice = Chosen::default();
        for kind in [
            Some(CommandKind::Skill),
            Some(CommandKind::Command),
            Some(CommandKind::Unknown),
            None,
        ] {
            let commands = [AgentCommand {
                name: "q".parse().unwrap(),
                description: String::new(),
                hint: None,
                kind,
            }];
            let inline = Menu::of("use /q", 6, Target::Session, Some(&commands), &choice).unwrap();
            assert_eq!(
                inline.rows.len(),
                usize::from(kind == Some(CommandKind::Skill))
            );
            if let Some(row) = inline.taken(&choice) {
                assert_eq!(row.name(), "q");
                assert_eq!(row.client(), None);
            }
            let root = Menu::of("/q", 2, Target::Session, Some(&commands), &choice).unwrap();
            assert_eq!(
                root.taken(&choice).unwrap().client(),
                Some(ClientCommand::Exit)
            );
        }
    }
}
