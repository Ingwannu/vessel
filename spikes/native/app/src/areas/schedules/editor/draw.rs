//! Source field order and spacing; all chrome comes from the shared kit.
use super::{
    AnyElement, Context, Picker, Rc, Shell, Tokens, Window, choices, close, io, kit, save,
};
use kit::{
    ButtonConfig, ButtonContent, ButtonKind, ButtonState, ControlSizing, Type, TypeExt as _,
    dialog::{DialogContent, DialogSize},
    form_field::{self, Content},
    theme::spacing,
    ui::{
        Div, IntoElement as _, ParentElement as _, SharedString, StatefulInteractiveElement as _,
        Styled as _, div, px,
    },
};
use std::sync::Arc;
pub(super) fn shared(text: &Arc<str>) -> SharedString {
    Arc::clone(text).into()
}
pub(super) fn framed(
    label: SharedString,
    body: AnyElement,
    error: Option<SharedString>,
    tokens: &Tokens,
) -> Div {
    form_field::frame(
        Content {
            label,
            body,
            trailing: None,
            hint: None,
            error,
        },
        tokens,
    )
}
pub(in crate::areas::schedules) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.schedules;
    let ui = &area.ui.editor;
    if !ui.mounted {
        return None;
    }
    let visit = ui.visit();
    let bootstrap = ui
        .bootstrap
        .as_ref()
        .map(|value| Rc::downgrade(&value.cycle));
    let cache = area
        .data
        .editor_cache
        .as_ref()
        .ok()
        .and_then(Option::as_ref)
        .filter(|form| Some(form.visit) == visit);
    let title = cache.map_or_else(
        || shell.words.schedules().form_new_title().into(),
        |form| shared(&form.title),
    );
    let footer = footer(shell, cache, visit, bootstrap.clone(), touch, tokens, cx);
    let body = if bootstrap.is_some() {
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::S4))
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.foreground_muted)
                    .whitespace_normal()
                    .child(shell.words.schedules().form_viewer_unknown()),
            )
            .child(select(
                shell,
                Selection::new(
                    Picker::Zone,
                    kit::select::FormValue::Placeholder(
                        shell.words.schedules().form_zone_placeholder().into(),
                    ),
                ),
                tokens,
                touch,
                cx,
            ))
            .into_any_element()
    } else {
        super::body::render(shell, tokens, touch, cx)
    };
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S4))
        .children(
            ui.opening_error()
                .map(|notice| super::super::draw::notice(notice, tokens)),
        )
        .child(body)
        .into_any_element();
    Some(kit::dialog(
        &ui.dialog,
        DialogContent {
            title,
            body,
            footer: vec![footer],
            size: DialogSize::Normal,
        },
        tokens,
        window,
        touch,
        move |shell, window, cx| close(shell, visit, bootstrap.clone(), window, cx),
        cx,
    ))
}
fn footer(
    shell: &Shell,
    cache: Option<&vessel_app_view::schedules::editor::presentation::Form>,
    visit: Option<super::Visit>,
    bootstrap: Option<super::Weak<()>>,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let saving = cache.is_some_and(|form| {
        form.submission == vessel_app_view::schedules::editor::presentation::Submission::Pending
    });
    let can_save = cache.is_some_and(|form| {
        form.submission == vessel_app_view::schedules::editor::presentation::Submission::Ready
    });
    let sizing = ControlSizing::from_compact(touch);
    let close_bootstrap = bootstrap;
    let cancel = kit::button_control(
        "schedule-form-cancel",
        ButtonContent::Label(shell.words.schedules().cancel().into()),
        ButtonConfig::new(
            ButtonKind::Secondary,
            if saving {
                ButtonState::Disabled
            } else {
                ButtonState::Enabled
            },
            sizing,
        ),
        tokens,
    )
    .flex_1()
    .on_click(cx.listener(move |shell, _, window, cx| {
        close(shell, visit, close_bootstrap.clone(), window, cx)
    }));
    let config = ButtonConfig::new(
        ButtonKind::Primary,
        if can_save {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        sizing,
    );
    let config = if saving { config.loading() } else { config };
    let save_button = kit::button_control(
        "schedule-form-save",
        ButtonContent::Label(cache.map_or_else(
            || shell.words.schedules().form_create().into(),
            |form| shared(&form.submit),
        )),
        config,
        tokens,
    )
    .flex_1()
    .on_click(cx.listener(move |shell, _, window, cx| {
        if let Some(visit) = visit {
            save(shell, visit, window, cx);
        }
    }));
    div()
        .flex()
        .flex_1()
        .gap(px(spacing::S3))
        .child(cancel)
        .child(save_button)
        .into_any_element()
}
pub(super) struct Selection {
    picker: Picker,
    value: kit::select::FormValue,
    leading: Option<AnyElement>,
    unavailable: bool,
}
impl Selection {
    pub(super) fn new(picker: Picker, value: kit::select::FormValue) -> Self {
        Self {
            picker,
            value,
            leading: None,
            unavailable: false,
        }
    }
    pub(super) fn leading(mut self, leading: Option<AnyElement>) -> Self {
        self.leading = leading;
        self
    }
    pub(super) fn unavailable(mut self, unavailable: bool) -> Self {
        self.unavailable = unavailable;
        self
    }
}
pub(super) fn select(
    shell: &Shell,
    selection: Selection,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let Selection {
        picker,
        value,
        leading,
        unavailable,
    } = selection;
    let ui = &shell.areas.schedules.ui.editor;
    let visit = ui.visit();
    let bootstrap = ui
        .bootstrap
        .as_ref()
        .map(|value| Rc::downgrade(&value.cycle));
    let disabled =
        unavailable || (bootstrap.is_none() && shell.areas.schedules.data.editor.blocked());
    let anchor = ui.anchors.get(picker);
    let loading = shell.areas.schedules.data.editor.operations().any(|slot| {
        slot.pending()
            && slot
                .request()
                .is_some_and(|request| match request.operation() {
                    io::Operation::Targets => matches!(picker, Picker::Project | Picker::Model),
                    io::Operation::Options(_) => {
                        matches!(picker, Picker::Model | Picker::Thinking | Picker::Mode)
                    }
                    io::Operation::Preview(_) | io::Operation::Save(_) => false,
                })
    });
    let trigger = kit::select::form_trigger(
        picker.id(),
        kit::select::FormContent {
            value,
            leading,
            sizing: ControlSizing::from_compact(touch),
            availability: if disabled {
                ButtonState::Disabled
            } else {
                ButtonState::Enabled
            },
            visibility: if ui.picking == Some(picker) && ui.picker.is_open() {
                kit::select::Visibility::Open
            } else {
                kit::select::Visibility::Closed
            },
            activity: if loading {
                kit::select::FormActivity::Loading
            } else {
                kit::select::FormActivity::Idle
            },
        },
        anchor,
        tokens,
    );
    let trigger = if disabled {
        trigger
    } else {
        trigger.on_click(cx.listener(move |shell, _, window, cx| {
            choices::open(shell, picker, visit, bootstrap.clone(), window, cx)
        }))
    };
    if matches!(picker, Picker::Cadence | Picker::Unit) {
        div().child(trigger)
    } else {
        framed(
            choices::label(shell, picker).to_owned().into(),
            trigger.into_any_element(),
            None,
            tokens,
        )
    }
}
