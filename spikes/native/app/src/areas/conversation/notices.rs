//! Canonical notices share kit alerts; plugin data stays opaque and unavailable
//! until the plugin owner's renderer is present. Compaction is a persistent marker.

use super::entries::{Paint, id};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, IntoElement as _, ParentElement as _, Styled as _, Window, div,
    px,
};
use vessel_app_kit::{Type, TypeExt as _, plain_text};
use vessel_proto::timeline::{
    Compaction, CompactionStatus, NotificationLevel, PluginTimelineItem, TimelineNotification,
};

/// Paseo `message.tsx` compaction caption uses 13 logical points.
const MARKER_TEXT_SIZE: f32 = 13.0;

pub(super) fn notification(
    paint: &Paint<'_>,
    index: usize,
    notice: &TimelineNotification,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let variant = match notice.level {
        NotificationLevel::Info => vessel_app_kit::alert::Variant::Info,
        NotificationLevel::Warning => vessel_app_kit::alert::Variant::Warning,
        NotificationLevel::Error => vessel_app_kit::alert::Variant::Error,
    };
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: None,
            description: Some(plain_text(
                ElementId::NamedInteger("timeline-notice".into(), id(index)),
                &notice.message,
                paint
                    .entries
                    .get(index)
                    .map_or(0, |entry| u64::from(entry.rev)),
                window,
                cx,
            )),
            action: None,
        },
        variant,
        vessel_app_kit::alert::Size::Small,
        paint.tokens,
    )
    .into_any_element()
}

pub(super) fn compaction(paint: &Paint<'_>, index: usize, item: &Compaction) -> AnyElement {
    let color = paint.tokens.neutral.foreground_muted;
    let mark = match item.status {
        CompactionStatus::Loading => vessel_app_kit::slider::spinner(
            ElementId::NamedInteger("compaction-loading".into(), id(index)),
            spacing::S3_5,
            color,
        )
        .into_any_element(),
        CompactionStatus::Completed => {
            glyph(Glyph::Scissors, spacing::S3, color).into_any_element()
        }
    };
    let line = || {
        div()
            .flex_1()
            .border_t_1()
            .border_color(paint.tokens.neutral.border)
    };
    div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .py(px(spacing::S3))
        .px(px(spacing::S4))
        .child(line())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::S2))
                .child(mark)
                .child(
                    div()
                        .type_(Type::Meta)
                        .text_size(px(vessel_app_kit::fonts::ui(MARKER_TEXT_SIZE)))
                        .text_color(color)
                        .child(paint.catalog.timeline_items().compaction(item).into_owned()),
                ),
        )
        .child(line())
        .into_any_element()
}

pub(super) fn plugin_unavailable(paint: &Paint<'_>, item: &PluginTimelineItem) -> AnyElement {
    // The data payload is intentionally not read or copied into this fallback.
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: Some(paint.catalog.timeline_items().plugin_unavailable().into()),
            description: Some(
                div()
                    .child(format!("{} · {}", item.plugin.as_str(), item.kind.as_str()))
                    .into_any_element(),
            ),
            action: None,
        },
        vessel_app_kit::alert::Variant::Info,
        vessel_app_kit::alert::Size::Small,
        paint.tokens,
    )
    .into_any_element()
}
