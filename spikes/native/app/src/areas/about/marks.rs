//! Vendor attributions use the existing About settings rows and links.
//! These are embedded trusted addresses; no wire-provided URL is opened.
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, outline_button, settings, theme::spacing};
use vessel_app_view::mark_attributions::MARKS;

pub(super) fn section(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.marks();
    let mut card = settings::card(tokens);
    for (index, mark) in MARKS.into_iter().enumerate() {
        let controls = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(spacing::S2))
            .child(link(
                format!("mark-source-{index}"),
                words.source,
                mark.source,
                tokens,
                cx,
            ))
            .child(link(
                format!("mark-terms-{index}"),
                words.terms,
                mark.terms,
                tokens,
                cx,
            ))
            .children(mark.licence.map(|url| {
                link(
                    format!("mark-licence-{index}"),
                    words.licence,
                    url,
                    tokens,
                    cx,
                )
            }))
            .into_any_element();
        card = card.child(settings::row_at(
            mark.name,
            Some(words.trademark(mark.owner).into()),
            None,
            Some(controls),
            index == 0,
            compact,
            tokens,
        ));
    }
    settings::section(
        words.title,
        div().child(card).child(
            div()
                .mt(px(spacing::S2))
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.purpose),
        ),
        tokens,
    )
}
fn link(
    id: String,
    label: &'static str,
    url: &'static str,
    tokens: &Tokens,
    cx: &Context<'_, Shell>,
) -> AnyElement {
    outline_button(ElementId::Name(id.into()), label, true, tokens)
        .on_click(cx.listener(move |_shell, _event, _window, cx| cx.open_url(url)))
        .into_any_element()
}
