//! `HostFilter` and Active/Ended segments use the shared selector surfaces.
use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::{
    self as kit, ComboHandlers, ComboHeader, ComboRow, ComboboxContent, ComboboxWidth, Leading,
    Tokens,
    segmented::{SegmentSize, segment_at, segmented_at},
    theme::spacing,
    ui::{
        AnyElement, Context, Div, IntoElement as _, ParentElement as _,
        StatefulInteractiveElement as _, Styled as _, Window, div, px,
    },
};
use vessel_app_view::schedules::{
    Bucket,
    presentation::{self, MachineLeading},
};
fn leading(value: MachineLeading, tokens: &Tokens) -> AnyElement {
    match value {
        MachineLeading::All => kit::picker::glyph(
            kit::assets::Glyph::Server,
            kit::theme::icon::SM,
            tokens.neutral.foreground_muted,
        )
        .into_any_element(),
        MachineLeading::Status(variant) => {
            kit::dot(status_color(variant, tokens)).into_any_element()
        }
    }
}
pub(super) fn status_color(
    variant: vessel_app_view::badge::Variant,
    tokens: &Tokens,
) -> kit::ui::Hsla {
    match variant {
        vessel_app_view::badge::Variant::Success => tokens.status().success.foreground,
        vessel_app_view::badge::Variant::Warning => tokens.status().warning.foreground,
        vessel_app_view::badge::Variant::Error => tokens.status().error.foreground,
        vessel_app_view::badge::Variant::Muted => tokens.neutral.foreground_muted,
    }
}
pub(super) fn toolbar(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let area = &shell.areas.schedules;
    let page = &area.data.page;
    let mut controls = div().flex().flex_wrap().items_center().gap(px(spacing::S3));
    if page.multiple_machines() {
        controls = controls.child(
            kit::select::host_filter(
                "schedules-host",
                page.machine_label(shell.words).to_owned(),
                leading(
                    presentation::machine_leading(&page.machine, &shell.fleet),
                    tokens,
                ),
                &area.ui.anchor,
                tokens,
            )
            .on_click(cx.listener(|shell, _, window, cx| {
                let ui = &mut shell.areas.schedules.ui;
                let _removed = ui.pending_focus.remove(&super::Popup::Hosts);
                ui.menu.close_and_restore(window, cx);
                ui.picker.set_anchor(ui.anchor.get());
                ui.picker.open(None, window, cx);
                cx.notify();
            })),
        );
    }
    let sizing = kit::ControlSizing::from_compact(touch);
    let mut status = segmented_at(SegmentSize::Medium, sizing);
    for (index, bucket) in page.buckets().into_iter().enumerate() {
        let words = shell.t.lang.schedules();
        let label = match bucket {
            Bucket::Active => words.active,
            Bucket::Ended => words.ended,
            Bucket::Unknown => words.unknown_filter,
        };
        status = status.child(
            segment_at(
                ("schedule-filter", index),
                label,
                page.bucket == bucket,
                true,
                SegmentSize::Medium,
                sizing,
                tokens,
            )
            .on_click(cx.listener(move |shell, _, window, cx| {
                if shell.areas.schedules.data.page.bucket == bucket {
                    return;
                }
                shell.areas.schedules.ui.menu.close_and_restore(window, cx);
                shell.areas.schedules.data.page.bucket = bucket;
                let _changed = shell.schedules_changed(cx);
                cx.notify();
            })),
        );
    }
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(spacing::S3))
        .child(controls.child(status).flex_shrink(1.0).min_w_0())
        .child(super::draw::new_action(
            shell,
            "schedules-new",
            tokens,
            touch,
            cx,
        ))
}
pub(super) fn popup(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.schedules;
    let choices = area.data.page.machine_choices(shell.words, "");
    let rows: Vec<_> = choices
        .iter()
        .map(|choice| ComboRow {
            checked: choice.checked,
            leading: Some(
                match presentation::machine_leading(&choice.filter, &shell.fleet) {
                    MachineLeading::All => Leading::SmallGlyph(kit::assets::Glyph::Server),
                    MachineLeading::Status(variant) => Leading::Dot(status_color(variant, tokens)),
                },
            ),
            ..ComboRow::new(choice.label.clone())
        })
        .collect();
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.schedules.ui.picker,
        chose: Rc::new(move |shell, index, window, cx| {
            if let Some(choice) = choices.get(index) {
                shell.areas.schedules.ui.menu.close_and_restore(window, cx);
                shell.areas.schedules.data.page.machine = choice.filter.clone();
                let _changed = shell.schedules_changed(cx);
            }
            shell.areas.schedules.ui.picker.close();
            cx.notify();
        }),
        back: None,
    };
    let header = ComboHeader {
        title: Some(shell.words.schedules().filter_by_machine().into()),
        back: false,
        search: false,
        empty: shell.words.schedules().search_empty().into(),
        note: None,
        side: kit::picker::Side::Below,
    };
    kit::combobox_with(
        ComboboxContent::new("schedules-hosts", &header, &rows, ComboboxWidth::Content),
        &area.ui.picker,
        &handlers,
        tokens,
        window,
        cx,
    )
}
