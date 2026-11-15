//! Release history uses bundled Markdown plus the existing signed feed's notes.

use crate::shell::Shell;
use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
use vessel_app_kit::theme::spacing::{S2, S4, S8};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, markdown, outline_button};
use vessel_app_view::desktop::changelog::{self, Release, Revealed};

const BUNDLED: &str = include_str!("../../../../CHANGELOG.md");

pub(super) struct History {
    releases: Vec<Release>,
    revealed: Revealed,
}

impl Default for History {
    fn default() -> Self {
        Self {
            releases: changelog::parse(BUNDLED),
            revealed: Revealed::default(),
        }
    }
}

impl Shell {
    pub(crate) fn open_changelog(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let mut releases = changelog::parse(BUNDLED);
        if let Some(notes) = self.running_notes() {
            let mut current = changelog::parse(&format!("## {}\n\n{notes}", crate::LABEL));
            releases.retain(|release| release.version() != crate::LABEL);
            current.extend(releases);
            releases = current;
        }
        self.areas.about.history = History {
            releases,
            revealed: Revealed::default(),
        };
        self.areas.about.notes = true;
        self.areas.about.notes_dialog.focus(window, cx);
        cx.notify();
        // The first release renders before the rest of the initial page. This
        // yields to GPUI rather than introducing a timer or continuous animation.
        cx.on_next_frame(window, |shell, _window, cx| {
            shell.areas.about.history.revealed.settled();
            cx.notify();
        });
    }
}

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let about = &shell.areas.about;
    if !about.notes {
        return None;
    }
    let words = shell.words.desktop_shell();
    let history = &about.history;
    let mut body = div().flex().flex_col().gap(px(S8)).child(
        div()
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .child(words.release_source().to_owned()),
    );
    for (index, release) in history
        .releases
        .iter()
        .take(history.revealed.count())
        .enumerate()
    {
        body = body.child(release_view(index, release, shell, tokens));
    }
    if history.releases.is_empty() {
        body = body.child(
            div()
                .type_(Type::Body)
                .child(words.no_release_notes().to_owned()),
        );
    }
    let footer = if history.revealed.count() < history.releases.len() {
        vec![
            outline_button(
                "changelog-show-more",
                words.show_more().to_owned(),
                true,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, _window, cx| {
                shell.areas.about.history.revealed.show_more();
                cx.notify();
            }))
            .into_any_element(),
        ]
    } else {
        Vec::new()
    };
    Some(dialog(
        &about.notes_dialog,
        DialogContent {
            title: words.whats_new().to_owned().into(),
            body: body.into_any_element(),
            footer,
            size: DialogSize::Wide,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        |shell: &mut Shell, _window, _cx| shell.areas.about.notes = false,
        cx,
    ))
}

fn release_view(index: usize, release: &Release, shell: &Shell, tokens: &Tokens) -> Div {
    let words = shell.words.desktop_shell();
    let installed = if release.version() == crate::LABEL {
        Some(vessel_app_kit::badge::badge(
            words.installed().to_owned(),
            vessel_app_kit::badge::Variant::Muted,
            vessel_app_kit::badge::Size::Small,
            None,
            tokens,
        ))
    } else {
        None
    };
    div()
        .flex()
        .flex_col()
        .gap(px(S4))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(S2))
                .pb(px(S2))
                .border_b_1()
                .border_color(tokens.neutral.border)
                .child(
                    div()
                        .type_(Type::Title)
                        .text_color(tokens.neutral.foreground)
                        .child(release.version().to_owned()),
                )
                .children(installed)
                .child(div().flex_1())
                .child(
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(release.date().to_owned()),
                ),
        )
        .child(markdown(
            ("changelog-release", index),
            release.markdown().to_owned(),
        ))
}
