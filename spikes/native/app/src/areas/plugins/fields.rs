use gpui::{EventEmitter, Render};
use vessel_app_kit::{
    TextField, Tokens,
    ui::{Context, IntoElement, ParentElement as _, Styled as _, Window, div, px},
};
use vessel_proto::plugins::{
    ButtonBehavior, Caption, ContentText, ContributionId, ControlContent, InvalidPluginValue,
    MenuEntry, Surface,
};

#[derive(Clone)]
pub(super) enum Event {
    Edit {
        identity: ContributionId,
        value: Result<ContentText, InvalidPluginValue>,
    },
    Submit {
        identity: ContributionId,
        value: Result<ContentText, InvalidPluginValue>,
    },
}

pub(super) struct Field {
    identity: ContributionId,
    label: Caption,
    pub(super) editor: TextField,
}
impl EventEmitter<Event> for Field {}
impl Field {
    pub(super) fn new(
        identity: ContributionId,
        label: Caption,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let editor = TextField::with_change(label.as_str(), Self::submit, Self::change, window, cx);
        Self {
            identity,
            label,
            editor,
        }
    }
    fn submit(&mut self, text: String, _window: &mut Window, cx: &mut Context<'_, Self>) {
        cx.emit(Event::Submit {
            identity: self.identity.clone(),
            value: text.parse(),
        });
    }
    fn change(&mut self, text: String, cx: &mut Context<'_, Self>) {
        cx.emit(Event::Edit {
            identity: self.identity.clone(),
            value: text.parse(),
        });
    }
}
impl Render for Field {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens: Tokens = vessel_app_kit::tokens(cx);
        let narrow = window.viewport_size().width < px(vessel_app_kit::picker::COMPACT);
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .child(vessel_app_kit::kicker(
                self.label.as_str().to_owned(),
                &tokens,
            ))
            .child(self.editor.element(&tokens, narrow, cx))
    }
}

pub(super) fn collect(surface: &Surface, fields: &mut Vec<(ContributionId, Caption)>) {
    for control in &surface.controls {
        match &control.content {
            ControlContent::TextField { label, .. } => {
                fields.push((control.id.clone(), label.clone()))
            }
            ControlContent::Stack { surface }
            | ControlContent::Row { surface }
            | ControlContent::Section { surface, .. } => collect(surface, fields),
            ControlContent::Button { button } => collect_behavior(&button.behavior, fields),
            ControlContent::Text { .. }
            | ControlContent::Markdown { .. }
            | ControlContent::Choice { .. }
            | ControlContent::Toggle { .. }
            | ControlContent::Status { .. } => {}
        }
    }
}
fn collect_behavior(behavior: &ButtonBehavior, fields: &mut Vec<(ContributionId, Caption)>) {
    match behavior {
        ButtonBehavior::Action { .. } => {}
        ButtonBehavior::Popover { surface } => collect(surface, fields),
        ButtonBehavior::Menu { items } => {
            for item in items {
                match item {
                    MenuEntry::Separator { .. } => {}
                    MenuEntry::Item { behavior, .. } => collect_behavior(behavior, fields),
                }
            }
        }
    }
}
