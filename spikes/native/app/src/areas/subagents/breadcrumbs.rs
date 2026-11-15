//! Navigation names the original provider path; old painted callbacks cannot retarget it.
use crate::shell::Shell;
use vessel_app_kit::theme::spacing::S1;
use vessel_app_kit::ui::{
    Context, Div, ElementId, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div,
    px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind};

pub(super) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let mut row = div().flex().flex_row().flex_wrap().gap(px(S1));
    let area = &shell.areas.subagents;
    let Some(track) = &area.track else {
        return row;
    };
    let Some(pane) = &area.pane else {
        return row;
    };
    let source = pane.identity();
    for (index, descriptor) in track.provider_path(&source.child).into_iter().enumerate() {
        let target = descriptor.id.clone();
        let expected = source.clone();
        let admission = track.target().clone();
        let revision = pane.projection_revision();
        let title = descriptor
            .title
            .as_ref()
            .or(descriptor.description.as_ref())
            .map_or(
                shell.words.subagents().untitled(),
                vessel_proto::subagents::ProviderText::as_str,
            );
        let current = target == source.child;
        row = row.child(
            button_with_kind(
                ElementId::NamedInteger("provider-path".into(), super::draw::row_number(index)),
                title.to_owned(),
                ButtonKind::Ghost,
                if current {
                    ButtonState::Disabled
                } else {
                    ButtonState::Enabled
                },
                tokens,
            )
            .on_click(cx.listener(move |shell, _, _, cx| {
                if !current
                    && shell
                        .areas
                        .subagents
                        .track
                        .as_ref()
                        .is_some_and(|track| track.target() == &admission)
                    && shell
                        .areas
                        .subagents
                        .pane
                        .as_ref()
                        .and_then(|pane| pane.projection_revision())
                        == revision
                    && shell
                        .areas
                        .subagents
                        .pane
                        .as_ref()
                        .map(vessel_app_view::subagents::ChildPane::identity)
                        == Some(expected.clone())
                {
                    shell.subagent_provider(&target);
                    cx.notify();
                }
            })),
        );
    }
    row
}
