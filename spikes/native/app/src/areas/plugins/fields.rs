use gpui::{EventEmitter, Render};
use vessel_app_kit::{
    TextField, Tokens,
    field::FieldId,
    ui::{Context, IntoElement, ParentElement as _, Styled as _, Window, div, px},
};
use vessel_proto::plugins::{
    Caption, ContentText, ContributionId, ControlContent, InvalidPluginValue, Surface,
};

#[derive(Clone)]
pub(super) enum Event {
    Edit {
        identity: ContributionId,
        source: FieldId,
        value: Result<ContentText, InvalidPluginValue>,
    },
    Submit {
        identity: ContributionId,
        source: FieldId,
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
        let editor = TextField::with_source(
            label.as_str(),
            |field, source, text, window, cx| {
                field.submit(source, &text, window, cx);
            },
            |field, source, text, cx| {
                field.change(source, &text, cx);
            },
            window,
            cx,
        );
        Self {
            identity,
            label,
            editor,
        }
    }
    fn submit(
        &mut self,
        source: FieldId,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        cx.emit(Event::Submit {
            identity: self.identity.clone(),
            source,
            value: text.parse(),
        });
    }
    fn change(&mut self, source: FieldId, text: &str, cx: &mut Context<'_, Self>) {
        cx.emit(Event::Edit {
            identity: self.identity.clone(),
            source,
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
                fields.push((control.id.clone(), label.clone()));
            }
            ControlContent::Stack { surface }
            | ControlContent::Row { surface }
            | ControlContent::Section { surface, .. } => collect(surface, fields),
            ControlContent::Button { .. }
            | ControlContent::Text { .. }
            | ControlContent::Markdown { .. }
            | ControlContent::Choice { .. }
            | ControlContent::Toggle { .. }
            | ControlContent::Status { .. } => {}
        }
    }
}
