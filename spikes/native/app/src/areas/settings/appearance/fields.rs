//! One live editor per field. Raw drafts, IME, selection and history belong
//! only to `TextField`; view supplies pure edit/commit/restore decisions.
use super::State;
use crate::shell::Shell;
use gpui::{App, Context, Window};
use vessel_app_kit::{TextField, field::FieldEvents};
use vessel_app_view::{
    appearance::form::{self, Field, FieldText},
    prefs::Appearance,
};

pub(super) struct Fields {
    interface_family: TextField,
    interface_size: TextField,
    content_size: TextField,
    mono_family: TextField,
    code_size: TextField,
    width: TextField,
}
impl Fields {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            interface_family: interface_family::new(window, cx),
            interface_size: interface_size::new(window, cx),
            content_size: content_size::new(window, cx),
            mono_family: mono_family::new(window, cx),
            code_size: code_size::new(window, cx),
            width: width::new(window, cx),
        }
    }
    pub(super) const fn of(&self, field: Field) -> &TextField {
        match field {
            Field::InterfaceFamily => &self.interface_family,
            Field::InterfaceSize => &self.interface_size,
            Field::ContentSize => &self.content_size,
            Field::MonoFamily => &self.mono_family,
            Field::CodeSize => &self.code_size,
            Field::Width => &self.width,
        }
    }
}
impl State {
    /// Rehydrate only changed saved axes; a theme switch cannot erase a draft.
    pub(super) fn sync(&mut self, saved: &Appearance, window: &mut Window, cx: &mut App) {
        for field in Field::ALL {
            let next = form::saved_text(field, saved);
            if self
                .saved
                .as_ref()
                .is_none_or(|before| form::saved_text(field, before) != next)
            {
                self.drafts.reset(field);
                self.fields.of(field).set_text(&display(next), window, cx);
            }
        }
        self.saved = Some(saved.clone());
        self.refresh_preview(saved, cx);
    }
}
fn display(text: FieldText) -> String {
    match text {
        FieldText::Value(value) => value.as_str().to_owned(),
        FieldText::WindowWidth => String::new(),
    }
}
impl Shell {
    fn appearance_edit(
        &mut self,
        field: Field,
        raw: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.appearance;
        let input = state.fields.of(field);
        // Leave the existing editor's marked text untouched until committed.
        if input.composing(window, cx) {
            return;
        }
        let edited = form::edit(field, raw);
        if edited.text.as_str() != raw {
            input.set_text(edited.text.as_str(), window, cx);
        }
        state.drafts.apply(edited.preview);
        state.refresh_preview(&self.prefs.settings.appearance, cx);
        cx.notify();
    }
    fn appearance_commit(
        &mut self,
        field: Field,
        raw: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self
            .areas
            .settings
            .appearance
            .fields
            .of(field)
            .composing(window, cx)
        {
            return;
        }
        let result = form::commit(field, raw, &self.prefs.settings.appearance);
        if let Some(change) = result.change {
            self.prefer(&change, window, cx);
        }
        let input = self.areas.settings.appearance.fields.of(field);
        let text = display(result.text);
        if input.text(cx) != text {
            input.set_text(&text, window, cx);
        }
        self.areas.settings.appearance.drafts.reset(field);
        self.areas
            .settings
            .appearance
            .refresh_preview(&self.prefs.settings.appearance, cx);
        cx.notify();
    }
    fn appearance_focus(field: Field, window: &mut Window, cx: &mut Context<'_, Self>) {
        match field {
            Field::InterfaceFamily | Field::MonoFamily => {}
            Field::InterfaceSize | Field::ContentSize | Field::CodeSize | Field::Width => {
                cx.defer_in(window, move |shell, window, cx| {
                    let input = shell.areas.settings.appearance.fields.of(field);
                    if input.focus_handle(cx).is_focused(window) {
                        input.select_all(window, cx);
                    }
                });
            }
        }
    }
}
// Function pointers are required by the shared field API. This one expansion
// binds each closed field identity without six copies of edit/commit logic.
macro_rules! binding {
    ($name:ident, $field:ident) => {
        mod $name {
            use super::*;
            pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> TextField {
                TextField::with_events(
                    "",
                    commit,
                    FieldEvents {
                        on_edit: Some(edit),
                        on_focus: Some(focus),
                        on_blur: Some(commit),
                    },
                    window,
                    cx,
                )
            }
            fn edit(
                shell: &mut Shell,
                raw: String,
                window: &mut Window,
                cx: &mut Context<'_, Shell>,
            ) {
                shell.appearance_edit(Field::$field, &raw, window, cx);
            }
            fn commit(
                shell: &mut Shell,
                raw: String,
                window: &mut Window,
                cx: &mut Context<'_, Shell>,
            ) {
                shell.appearance_commit(Field::$field, &raw, window, cx);
            }
            fn focus(_: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
                Shell::appearance_focus(Field::$field, window, cx);
            }
        }
    };
}
binding!(interface_family, InterfaceFamily);
binding!(interface_size, InterfaceSize);
binding!(content_size, ContentSize);
binding!(mono_family, MonoFamily);
binding!(code_size, CodeSize);
binding!(width, Width);

// Tests live with the private commit helper; the area exposes no test-only API.
#[cfg(test)]
mod tests;
