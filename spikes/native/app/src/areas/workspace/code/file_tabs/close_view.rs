//! Existing shared dialog, bound to the file owner's single original decision.
use super::{AnyElement, Context, LayoutKey, Shell, TabId, Tokens, Window, close::Decision};
use vessel_app_kit::{
    ButtonKind, ButtonState, Type, TypeExt as _, button, button_with_kind,
    dialog::{DialogContent, DialogSize, dialog as render_dialog},
    ui::{IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div},
};

pub(super) fn dialog(
    shell: &Shell,
    workspace: &LayoutKey,
    tab: TabId,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let prompt = shell
        .areas
        .workspace
        .code
        .file_tabs
        .close
        .as_ref()
        .filter(|prompt| prompt.workspace == *workspace && prompt.plan.tab() == tab)?;
    let ticket = prompt.ticket;
    let labels = shell.words().labels();
    let content = DialogContent {
        title: labels.file_close_title.into(),
        body: div()
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(labels.file_close_message)
            .into_any_element(),
        footer: vec![
            button("file-close-cancel", labels.cancel, false, tokens)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.file_close_answered(ticket, Decision::Keep, cx);
                }))
                .into_any_element(),
            button_with_kind(
                "file-close-discard",
                labels.file_close_discard,
                ButtonKind::Destructive,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.file_close_answered(ticket, Decision::Discard, cx);
            }))
            .into_any_element(),
        ],
        size: DialogSize::Normal,
    };
    Some(render_dialog(
        &prompt.dialog,
        content,
        tokens,
        window,
        vessel_app_kit::breakpoint::phone(window),
        move |shell: &mut Shell, _, cx| shell.file_close_answered(ticket, Decision::Keep, cx),
        cx,
    ))
}
