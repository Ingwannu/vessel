//! `CatalogTile`'s order and exact shared controls; no area-owned button or link.
use super::*;
use vessel_app_kit::ui::{FluentBuilder as _, StatefulInteractiveElement as _, canvas};
use vessel_app_kit::{
    ButtonConfig, ButtonContent, ButtonKind, ButtonSize, ButtonState, ControlSizing, assets::Glyph,
    badge, button_control, glyph,
};
use vessel_app_view::agent_status::{
    addition::{self, Control},
    catalogue::{
        Offer,
        grid::{Grid, Rail},
    },
};
use vessel_proto::agents::CatalogueEntry;

const ICON_FRAME: f32 = 36.0;
const FALLBACK_ICON: f32 = 20.0;
pub(super) fn grid<'a>(
    entries: impl Iterator<Item = &'a CatalogueEntry>,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let width = Rc::clone(&shell.areas.settings.agents.catalogue.width);
    let layout = Grid::resolve(Rail::measured_points(f32::from(width.get()), spacing::S3));
    let probe = Rc::clone(&width);
    div()
        .relative()
        .w_full()
        .flex()
        .flex_wrap()
        .gap(px(spacing::S3))
        .child(
            canvas(
                move |bounds, window, _| {
                    if probe.get() != bounds.size.width {
                        probe.set(bounds.size.width);
                        window.refresh();
                    }
                },
                |_, (), _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
        .children(entries.enumerate().map(|(index, entry)| {
            let tile = tile(index, entry, shell, tokens, compact, cx);
            match layout {
                Grid::Single => tile.w_full(),
                Grid::Columns(columns) => tile.w(px(columns.tile_width().points())),
            }
        }))
}
fn tile(
    index: usize,
    entry: &CatalogueEntry,
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.agent_catalog();
    let page = super::super::page(shell);
    let inventory = page.and_then(|page| page.inventory());
    let agents = inventory.map_or(&[][..], |inventory| inventory.agents.as_slice());
    let offer = catalogue::offer(entry, agents);
    let mut meta = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S2))
        .child(badge::badge(
            words.type_label(),
            vessel_app_kit::badge::Variant::Muted,
            badge::Size::ExtraSmall,
            None,
            tokens,
        ));
    if let Some(resource) = catalogue::resource(entry) {
        let label = words.resource_label(&resource);
        let url = resource.url().clone();
        meta = meta.child(
            vessel_app_kit::external_link::external_link(
                format!("provider-resource-{index}"),
                Some(label.into()),
                tokens,
            )
            .on_click(cx.listener(move |_, _, _, cx| cx.open_url(url.as_str()))),
        );
    }
    let mut details = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(entry.name.clone()),
        )
        .child(meta);
    // Eligibility and the missing runner are node facts. Keep their whole
    // instruction beside the offer, without claiming that Add installs it.
    if !matches!(offer, Offer::Add(catalogue::Runner::Found)) {
        details = details.child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.offer(&offer).to_owned()),
        );
    }
    let mut tile = settings::card(tokens)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S3))
        .p(px(spacing::S3))
        .child(
            div()
                .flex_none()
                .size(px(ICON_FRAME))
                .rounded(px(vessel_app_kit::theme::radius::MD))
                .bg(tokens.neutral.surface2)
                .flex()
                .items_center()
                .justify_center()
                .child(glyph(
                    Glyph::PackagePlus,
                    FALLBACK_ICON,
                    tokens.neutral.foreground,
                )),
        )
        .child(details);
    match offer {
        Offer::Add(_) => {
            let control = addition::registry_control(page, entry);
            let adding = control == Control::Adding;
            let config = ButtonConfig::new(
                ButtonKind::Outline,
                if control == Control::Ready {
                    ButtonState::Enabled
                } else {
                    ButtonState::Disabled
                },
                ControlSizing::from_compact(compact),
            )
            .with_size(ButtonSize::Small);
            let config = if adding { config.loading() } else { config };
            let original = entry.clone();
            let scope = shell.areas.settings.agents.machine.clone();
            tile = tile.child(
                button_control(
                    format!("provider-add-{index}"),
                    ButtonContent::Label(if adding { words.adding() } else { words.add() }.into()),
                    config,
                    tokens,
                )
                .on_click(cx.listener(move |shell, _, window, cx| {
                    if shell.agent_context(scope.as_ref()) {
                        shell.agent_open_add(Some(original.clone()), window, cx);
                    }
                }))
                .flex_shrink_0(),
            );
        }
        Offer::Recorded(_) => {
            tile = tile.child(badge::badge(
                words.added_label(),
                vessel_app_kit::badge::Variant::Muted,
                badge::Size::ExtraSmall,
                None,
                tokens,
            ))
        }
        Offer::BuiltIn(_) | Offer::Download(_) | Offer::Unavailable => {}
    }
    tile
}
