//! Per-open kit admission plus original import scope, never a late alias lookup.
mod choices;
mod retirement;
use super::Picker;
use crate::shell::Shell;
use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_kit::{ComboHandlers, Tokens, combobox};
use vessel_app_view::history::Loading;

/// UI availability is not authority; effects still cross the selected link fence.
pub(super) fn ready(shell: &Shell) -> bool {
    let area = &shell.areas.import;
    area.open
        && !area.model.importing()
        && !matches!(area.model.loading(), Loading::Pending(_))
        && area.profile_request.is_none()
        && area
            .model
            .scope()
            .is_none_or(|scope| choices::current(shell, scope))
}

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.import;
    let which = area.choosing?;
    let (scopes, rows): (Vec<_>, Vec<_>) = choices::rows(shell, which).into_iter().unzip();
    let capture = super::intent::Capture::of(shell);
    combobox(
        "history-picker",
        &area.picker,
        &vessel_app_kit::select::header(),
        &rows,
        &ComboHandlers {
            state: |shell: &mut Shell| &mut shell.areas.import.picker,
            chose: std::rc::Rc::new(move |shell: &mut Shell, index, window, cx| {
                if !capture.accepts(shell)
                    || shell.areas.import.choosing != Some(which)
                    || !ready(shell)
                {
                    return;
                }
                let Some(Some(scope)) = scopes.get(index) else {
                    return;
                };
                if !choices::current(shell, scope) {
                    return;
                }
                // Close only this admitted opening; the kit preserves newer focus.
                shell.areas.import.picker.close_and_restore(window, cx);
                shell.history_choose_scope(scope.clone());
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
    pub(super) fn history_picker(
        &mut self,
        which: Picker,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !ready(self) {
            return;
        }
        let area = &mut self.areas.import;
        let anchor = match which {
            Picker::Machine => &area.machine_anchor,
            Picker::Agent => &area.agent_anchor,
            Picker::Account => &area.account_anchor,
        };
        area.picker.set_anchor(anchor.get());
        area.choosing = Some(which);
        area.picker.open(None, window, cx);
        cx.notify();
    }
    fn history_choose_scope(&mut self, scope: vessel_app_view::history::Scope) {
        if !ready(self) || !choices::current(self, &scope) {
            return;
        }
        let agent_changed = self
            .areas
            .import
            .model
            .scope()
            .is_none_or(|old| old.context != scope.context || old.harness != scope.harness);
        if !self.areas.import.advance_visit() {
            return;
        }
        self.areas.import.model.choose(scope);
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
        InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _,
        div, px,
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
                ready(shell),
                crate::areas::draft::compact(window),
                tokens,
            )
            .debug_selector(move || format!("history-{which:?}"))
            .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
            .on_click(cx.listener(move |shell, _, window, cx| listener(shell, window, cx))),
        )
}
