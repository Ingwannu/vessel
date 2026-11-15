//! Existing combobox rows bind to typed choices, never display strings.

use super::Picker;
use crate::shell::Shell;
use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_kit::{ComboHandlers, ComboRow, Tokens, combobox};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::history::Scope;
use vessel_proto::{HarnessRef, ProfileName};

#[derive(Debug, Clone)]
enum Choice {
    Machine(MachineKey),
    Agent(HarnessRef),
    Account(Option<ProfileName>),
}

fn choices(shell: &Shell, which: Picker) -> Vec<(Choice, ComboRow)> {
    let scope = shell.areas.import.model.scope();
    match which {
        Picker::Machine => shell
            .fleet
            .online()
            .map(|machine| {
                let mut row = ComboRow::new(machine.name.clone());
                row.checked = scope.is_some_and(|scope| scope.machine.as_str() == machine.name);
                row.disabled = !machine.agents.iter().any(vessel_app_view::Agent::available);
                (Choice::Machine(MachineKey::from_book(&machine.name)), row)
            })
            .collect(),
        Picker::Agent => scope
            .and_then(|scope| {
                shell
                    .fleet
                    .online()
                    .find(|machine| machine.name == scope.machine.as_str())
            })
            .into_iter()
            .flat_map(|machine| &machine.agents)
            .map(|agent| {
                let mut row = ComboRow::new(agent.name.clone());
                row.checked = scope.is_some_and(|scope| scope.harness == agent.harness);
                row.disabled = !agent.available();
                (Choice::Agent(agent.harness.clone()), row)
            })
            .collect(),
        Picker::Account => {
            let mut row = ComboRow::new(shell.words.history().default_account());
            row.checked = scope.is_some_and(|scope| scope.profile.is_none());
            let mut rows = vec![(Choice::Account(None), row)];
            if let Ok(profiles) = &shell.areas.import.profiles {
                rows.extend(profiles.iter().map(|profile| {
                    let mut row = ComboRow::new(
                        profile
                            .label
                            .clone()
                            .unwrap_or_else(|| profile.name.to_string()),
                    );
                    row.checked =
                        scope.is_some_and(|scope| scope.profile.as_ref() == Some(&profile.name));
                    (Choice::Account(Some(profile.name.clone())), row)
                }));
            }
            rows
        }
    }
}

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let which = shell.areas.import.choosing?;
    let (choices, rows): (Vec<_>, Vec<_>) = choices(shell, which)
        .into_iter()
        .map(|(choice, row)| ((choice, row.disabled), row))
        .unzip();
    let capture = super::intent::Capture::of(shell);
    combobox(
        "history-picker",
        &shell.areas.import.picker,
        &vessel_app_kit::select::header(),
        &rows,
        &ComboHandlers {
            state: |shell: &mut Shell| &mut shell.areas.import.picker,
            chose: std::rc::Rc::new(move |shell: &mut Shell, index, window, cx| {
                if !capture.accepts(shell) || shell.areas.import.choosing != Some(which) {
                    return;
                }
                if let Some((choice, disabled)) = choices.get(index)
                    && !disabled
                {
                    if !shell.areas.import.advance_visit() {
                        return;
                    }
                    shell.history_choose(choice.clone());
                }
                shell.areas.import.picker.close();
                shell.areas.import.choosing = None;
                shell.areas.import.dialog.focus(window, cx);
                cx.notify();
            }),
            back: None,
        },
        vessel_app_kit::select::WIDTH,
        tokens,
        window,
        cx,
    )
}

impl Shell {
    fn history_choose(&mut self, choice: Choice) {
        if self.areas.import.model.importing() {
            return;
        }
        let scope = match choice {
            Choice::Machine(machine) => {
                let Some(harness) = self
                    .fleet
                    .online()
                    .find(|m| m.name == machine.as_str())
                    .and_then(|m| m.agents.iter().find(|a| a.available()))
                    .map(|a| a.harness.clone())
                else {
                    return;
                };
                let Some(context) = self
                    .fleet
                    .online()
                    .find(|m| m.name == machine.as_str())
                    .and_then(vessel_app_view::Machine::connection_context)
                else {
                    return;
                };
                Scope {
                    context,
                    machine,
                    harness,
                    profile: None,
                    cwd: None,
                }
            }
            Choice::Agent(harness) => {
                let Some(mut scope) = self.areas.import.model.scope().cloned() else {
                    return;
                };
                scope.harness = harness;
                scope.profile = None;
                scope
            }
            Choice::Account(profile) => {
                let Some(mut scope) = self.areas.import.model.scope().cloned() else {
                    return;
                };
                scope.profile = profile;
                scope
            }
        };
        let agent_changed = self
            .areas
            .import
            .model
            .scope()
            .is_none_or(|old| old.machine != scope.machine || old.harness != scope.harness);
        self.areas.import.choose(scope);
        if agent_changed {
            self.areas.import.profiles = Ok(Vec::new());
            self.history_profiles();
        }
        self.areas.import.reproject();
        self.history_refresh();
    }
}

/// Three controls share one typed picker while retaining separate anchors.
pub(super) fn controls(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::Div {
    use vessel_app_kit::theme::spacing::S2;
    use vessel_app_kit::ui::{ParentElement as _, Styled as _, div, px};
    let area = &shell.areas.import;
    let words = shell.words.history();
    let scope = area.model.scope();
    let machine = scope
        .map_or(words.choose_machine(), |scope| scope.machine.as_str())
        .to_owned();
    let agent = scope.map_or_else(
        || words.choose_agent().to_owned(),
        |scope| vessel_app_view::agent_name(&scope.harness),
    );
    let profile = scope
        .and_then(|scope| scope.profile.as_ref())
        .map_or_else(|| words.default_account().to_owned(), ToString::to_string);
    let fields = [
        (Picker::Machine, words.machine(), machine),
        (Picker::Agent, words.agent(), agent),
        (Picker::Account, words.account(), profile),
    ];
    div().flex().flex_col().gap(px(S2)).children(
        fields
            .into_iter()
            .map(|(which, label, value)| field((which, label, value), shell, tokens, window, cx)),
    )
}

fn field(
    (which, label, value): (Picker, &str, String),
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::Div {
    use vessel_app_kit::theme::spacing::S2;
    use vessel_app_kit::ui::{
        ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
    };
    use vessel_app_kit::{Type, TypeExt as _};
    let area = &shell.areas.import;
    let anchor = match which {
        Picker::Machine => &area.machine_anchor,
        Picker::Agent => &area.agent_anchor,
        Picker::Account => &area.account_anchor,
    };
    let listener = super::intent::listener(shell, super::intent::Action::Picker(which));
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(S2))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(label.to_owned()),
        )
        .child(
            vessel_app_kit::select::trigger(
                format!("history-{which:?}"),
                value,
                anchor,
                area.choosing == Some(which) && area.picker.is_open(),
                !area.model.importing(),
                crate::areas::draft::compact(window),
                tokens,
            )
            .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
            .on_click(cx.listener(move |shell, _, window, cx| listener(shell, window, cx))),
        )
}
