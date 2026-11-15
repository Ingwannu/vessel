//! Model, effort and mode use the same controls and kept choices as a new
//! chat. No parallel option cache or preference document lives here.

use std::rc::Rc;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, px,
};
use vessel_app_kit::{ComboHandlers, ComboRow, Tokens, combobox, outline_button, select, settings};
use vessel_app_link::Command;
use vessel_app_view::{Controls, Picks, SelectControl, choices_for, controls_of, with_picks};
use vessel_proto::HarnessRef;

use crate::shell::Shell;
use vessel_app_view::offers::Offered;

// The composer's model list is 360 wide; the 220-wide preference list
// cuts model names before a person can distinguish the available choices.
const CHOICES_WIDTH: f32 = 360.0;

fn controls(agent: &HarnessRef, shell: &Shell) -> Option<Controls> {
    let machine = shell.areas.settings.agents.machine.as_ref()?;
    let options = shell.offer_options(machine.as_str(), agent)?;
    let picks = shell
        .prefs
        .choices(machine.as_str(), agent)
        .map(|choices| Picks::from_choices(choices.to_vec()))
        .unwrap_or_default();
    Some(controls_of(
        &with_picks(options, &picks),
        &vessel_app_view::agent_name(agent),
        shell.t.lang,
    ))
}

fn slots(controls: &Controls) -> impl Iterator<Item = &SelectControl> {
    [&controls.model, &controls.effort, &controls.permissions]
        .into_iter()
        .filter_map(Option::as_ref)
}

pub(super) fn render(
    agent: &HarnessRef,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let state = &shell.areas.settings.agents;
    let words = shell.t.lang.agents();
    let mut card = settings::card(tokens)
        .child(super::super::note(words.defaults_note(), tokens).p(px(spacing::S4)));
    if let Some(controls) = controls(agent, shell) {
        let options_words = shell.t.lang.options();
        let named = [
            (&controls.model, options_words.model, &state.model_anchor),
            (&controls.effort, options_words.effort, &state.effort_anchor),
            (
                &controls.permissions,
                options_words.permissions,
                &state.mode_anchor,
            ),
        ];
        let mut any = false;
        for (select, title, anchor) in named {
            let Some(control) = select.as_ref().filter(|control| !control.values.is_empty()) else {
                continue;
            };
            any = true;
            let id = control.id.clone();
            let picker = select::trigger(
                format!("agent-default-{}", control.id),
                control.current_name().to_owned(),
                anchor,
                state.default_id.as_ref() == Some(&id) && state.defaults.is_open(),
                true,
                compact,
                tokens,
            )
            .on_click(cx.listener({
                let anchor = Rc::clone(anchor);
                move |shell, _, window, cx| {
                    let state = &mut shell.areas.settings.agents;
                    state.default_id = Some(id.clone());
                    state.defaults.set_anchor(
                        anchor
                            .get()
                            .map(|bounds| select::end_aligned(bounds, CHOICES_WIDTH)),
                    );
                    state.defaults.open(None, window, cx);
                    cx.notify();
                }
            }));
            card = card.child(settings::row_at(
                title,
                None,
                None,
                Some(picker.into_any_element()),
                false,
                compact,
                tokens,
            ));
        }
        if !any {
            card = card.child(super::super::note(words.no_options(), tokens).p(px(spacing::S4)));
        }
    } else {
        card = card.child(probe_message(agent, shell, tokens));
    }
    card = card.child(footer(agent, shell, tokens, compact, cx));
    settings::section(words.defaults(), card, tokens)
}

pub(crate) fn list(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.settings.agents;
    if !state.defaults.is_open() || !shell.agent_context(state.machine.as_ref()) {
        return None;
    }
    let agent = state
        .pages
        .get(state.machine.as_ref()?.as_str())?
        .selected()?;
    let controls = controls(agent, shell)?;
    let id = state.default_id.as_ref()?;
    let control = slots(&controls).find(|control| control.id == *id)?;
    let mut group = None;
    let rows: Vec<ComboRow> = control
        .values
        .iter()
        .map(|value| {
            let heading = if value.group == group {
                None
            } else {
                group.clone_from(&value.group);
                value.group.clone().map(Into::into)
            };
            ComboRow {
                checked: value.id == control.current,
                detail: value.description.clone().map(Into::into),
                heading,
                ..ComboRow::new(value.name.clone())
            }
        })
        .collect();
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.settings.agents.defaults,
        chose: Rc::new(|shell, index, _, cx| shell.agent_default_choose(index, cx)),
        back: None,
    };
    combobox(
        "agent-default-list",
        &state.defaults,
        &select::header(),
        &rows,
        &handlers,
        CHOICES_WIDTH,
        tokens,
        window,
        cx,
    )
}

impl Shell {
    fn agent_default_choose(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let state = &self.areas.settings.agents;
        if !state.defaults.is_open() || !self.agent_context(state.machine.as_ref()) {
            return;
        }
        let Some(agent) = state
            .machine
            .as_ref()
            .and_then(|machine| state.pages.get(machine.as_str()))
            .and_then(|page| page.selected())
            .cloned()
        else {
            return;
        };
        let Some(machine) = state.machine.clone() else {
            return;
        };
        let Some(controls) = controls(&agent, self) else {
            return;
        };
        let Some(control) =
            slots(&controls).find(|control| Some(&control.id) == state.default_id.as_ref())
        else {
            return;
        };
        let Some(value) = control.values.get(index) else {
            return;
        };
        let mut picks = self
            .prefs
            .choices(machine.as_str(), &agent)
            .map(|choices| Picks::from_choices(choices.to_vec()))
            .unwrap_or_default();
        picks.pick(control.choose(value.id.clone()));
        let Some(options) = self.offer_options(machine.as_str(), &agent) else {
            return;
        };
        let choices = choices_for(options, &picks);
        self.prefs.keep_choices(machine.as_str(), &agent, &choices);
        self.areas.settings.agents.defaults.close();
        self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
        cx.notify();
    }

    fn agent_defaults_reset(&mut self, agent: &HarnessRef, cx: &mut Context<'_, Self>) {
        if let Some(machine) = self.areas.settings.agents.machine.clone() {
            self.prefs.keep_choices(machine.as_str(), agent, &[]);
            self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
        }
        cx.notify();
    }
}

fn probe_message(agent: &HarnessRef, shell: &Shell, tokens: &Tokens) -> Div {
    let state = &shell.areas.settings.agents;
    let words = shell.t.lang.agents();
    let mut card = gpui::div();
    let message = state
        .machine
        .as_ref()
        .map(|machine| shell.offered(machine.as_str(), agent));
    match message {
        Some(Offered::Failed(error)) => {
            // The app's sentence; the machine's own words in the tooltip.
            let alert = vessel_app_kit::alert::alert(
                vessel_app_kit::alert::Content {
                    title: Some(words.options_failed().into()),
                    description: None,
                    action: None,
                },
                crate::badge::alert_variant(
                    vessel_app_view::agent_status::problem::Problem::Call.variant(),
                ),
                vessel_app_kit::alert::Size::Small,
                tokens,
            );
            card = card.child(super::super::with_original(alert, error.to_owned(), shell));
        }
        Some(Offered::Loading) | None => {
            card =
                card.child(super::super::note(words.options_loading(), tokens).p(px(spacing::S4)));
        }
        Some(Offered::Ready { .. }) => {}
    }
    card
}

fn footer(
    agent: &HarnessRef,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let state = &shell.areas.settings.agents;
    let words = shell.t.lang.agents();
    let mut actions = gpui::div()
        .p(px(spacing::S4))
        .flex()
        .flex_wrap()
        .gap(px(spacing::S2));
    let custom = state
        .machine
        .as_ref()
        .and_then(|machine| shell.prefs.choices(machine.as_str(), agent))
        .is_some_and(|choices| !choices.is_empty());
    if custom {
        let target = agent.clone();
        let scope = state.machine.clone();
        actions = actions.child(
            outline_button("reset-agent-defaults", words.own_defaults(), true, tokens)
                .when(compact, |button| {
                    button.min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
                })
                .on_click(cx.listener(move |shell, _, _, cx| {
                    if shell.agent_context(scope.as_ref()) {
                        shell.agent_defaults_reset(&target, cx);
                    }
                })),
        );
    }
    let loading = state
        .machine
        .as_ref()
        .is_some_and(|machine| shell.offers_asking(machine.as_str(), agent));
    let target = agent.clone();
    let scope = state.machine.clone();
    actions.child(
        outline_button(
            "reload-agent-choices",
            words.options_retry(),
            !loading,
            tokens,
        )
        .when(compact, |button| {
            button.min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
        })
        .on_click(cx.listener(move |shell, _, _, cx| {
            if !loading
                && shell.agent_context(scope.as_ref())
                && let Some(machine) = shell.areas.settings.agents.machine.clone()
            {
                shell.want_offers(
                    machine.as_str().to_owned(),
                    target.clone(),
                    vessel_app_view::offers::Why::Retry,
                );
            }
            cx.notify();
        })),
    )
}
