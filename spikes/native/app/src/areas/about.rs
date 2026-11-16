//! About, as Paseo's settings show it (`screens/settings-screen.tsx`
//! `AboutSection` and `DesktopAppUpdateRow`, `styles/settings.ts`): the
//! section "정보" with one card: the app's version on this device, the
//! app's updates (what the updater last did, "확인", and the update button
//! that restarts into a build in place), whether it updates by itself, and
//! the log file. Also the note shown once at the top of the frame after an
//! update ("Updated to 0.0.0.0.3", with the feed's notes), dismissible.
//!
//! [Decision Log]
//! - Purpose: the owner, on 0.0.0.0.5: the About screen was loose lines,
//!   a button and a chip ("꼬라지가 아주 개판"); it is Paseo's settings
//!   layout and Paseo's words now.
//! - Choice: the kit's settings furniture (`kit/src/settings.rs`); the
//!   page as Paseo's `PageLayout`: 16 around, 24 above, at most 720 wide;
//!   Paseo's update words for each of the updater's states; Paseo's
//!   outline button for "확인" and its accent button for the update.
//!   Paseo's confirm before the update restarts the app; "새로운 소식" opening
//!   the release notes; the log row's "경로 복사" and "로그 열기".
//! - Differences, each for a reason: "새로운 소식" lists bundled published
//!   Vessel release notes and supplements the running label with notes from
//!   the existing signed feed. Vessel has no published changelog website to
//!   fetch like Paseo's; the bundled source retains its known-limit notices.
//!   There is no release channel choice
//!   (one feed); a switch row for automatic updates, which Paseo does not
//!   have (Vessel's updater has the setting, track R's
//!   `Updater::set_automatic`); "로그 경로가 복사되었습니다." is said in the
//!   frame's foot.
//! - Settings › About (`areas/settings`) shows this card in Settings'
//!   page, then Vessel's licence and the parts the app is built from with
//!   theirs, as the page's About; its own screen and route are gone.

mod changelog;
mod marks;

use std::path::PathBuf;

use crate::update::UpdateState;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::ui::{
    AnyElement, ClipboardItem, Context, Div, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, button, button_with_kind, glyph,
    outline_button, settings,
};

use crate::shell::Shell;
use crate::update::Updated;
use vessel_app_view::about::{BUNDLED, VESSEL_LICENCE};

/// About's own state: its two dialogs, and the switch's changes (for its
/// motion).
pub(crate) struct AboutState {
    /// Times "자동으로 업데이트" was switched this run.
    pub(crate) automatic_turns: u64,
    /// The confirm before restarting into the build in place is open.
    confirm: bool,
    confirm_dialog: DialogState,
    /// The release notes are open.
    notes: bool,
    notes_dialog: DialogState,
    history: changelog::History,
}

impl AboutState {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            automatic_turns: 0,
            confirm: false,
            confirm_dialog: DialogState::new(cx),
            notes: false,
            notes_dialog: DialogState::new(cx),
            history: changelog::History::default(),
        }
    }
}

/// About's card: the version, what is new, the updates, the log.
fn card(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.update();
    let mut card = settings::card(tokens).child(settings::row(
        words.app_version(),
        Some(words.this_device().into()),
        Some(value(shell.updates.version(), tokens)),
        true,
        tokens,
    ));
    card = card.child(whats_new_row(shell, tokens, cx));
    if shell.updates_on() {
        card = card
            .child(updates_row(shell, tokens, cx))
            .child(automatic_row(shell, tokens, cx));
    } else {
        card = card.child(settings::row(
            words.app_updates(),
            Some(words.no_updates().into()),
            None,
            false,
            tokens,
        ));
    }
    card.child(logs_row(shell, tokens, cx))
}

/// About's dialogs, over the frame.
pub(crate) fn dialogs(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> impl Iterator<Item = AnyElement> {
    confirm_dialog(shell, tokens, window, cx)
        .into_iter()
        .chain(changelog::render(shell, tokens, window, cx))
}

/// About as Settings shows it (`areas/settings`): the card, then Vessel's
/// licence and the parts the app is built from with theirs, as the page's
/// About (`features/settings/about.tsx`); `compact` on a phone.
pub(crate) fn content(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let about = shell.words.about();
    let mut licences = settings::card(tokens).child(settings::row_at(
        "Vessel",
        Some(about.licence_note().into()),
        None,
        Some(value(VESSEL_LICENCE, tokens)),
        true,
        compact,
        tokens,
    ));
    for part in BUNDLED {
        licences = licences.child(settings::row_at(
            part.name,
            Some(about.role(part.role).into()),
            None,
            Some(value(part.licence, tokens)),
            false,
            compact,
            tokens,
        ));
    }
    let body = div()
        .flex()
        .flex_col()
        // The page's name for it: the page's title says About already.
        .child(settings::section("Vessel", card(shell, tokens, cx), tokens))
        // Paseo's "Connected hosts", Settings' (`settings/connected.rs`).
        .children(crate::areas::settings::connected::section(shell, tokens, compact))
        .child(settings::section(about.software(), licences, tokens))
        .child(marks::section(shell, tokens, compact, cx));
    #[cfg(not(target_arch = "wasm32"))]
    let body = body.child(crate::areas::desktop::cli_section(shell, tokens, cx));
    body.into_any_element()
}

/// "새로운 소식": Paseo's row with a chevron, the foreground's on hover.
fn whats_new_row(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.update();
    let neutral = tokens.neutral;
    div().child(
        settings::row(
            words.whats_new(),
            Some(words.whats_new_hint().into()),
            Some(glyph(Glyph::ChevronRight, 14.0, neutral.foreground_muted).into_any_element()),
            false,
            tokens,
        )
        .id("whats-new")
        .cursor_pointer()
        .on_click(cx.listener(|this, _event, window, cx| {
            this.open_changelog(window, cx);
        })),
    )
}

/// Paseo's confirm before the update installs and the app restarts.
fn confirm_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let about = &shell.areas.about;
    if !about.confirm {
        return None;
    }
    let words = shell.words.update();
    let footer = vec![
        button("install-cancel", words.cancel(), false, tokens)
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.areas.about.confirm = false;
                cx.notify();
            }))
            .into_any_element(),
        button("install-confirm", words.install_confirm(), true, tokens)
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.areas.about.confirm = false;
                this.restart_to_update(cx);
            }))
            .into_any_element(),
    ];
    Some(dialog(
        &about.confirm_dialog,
        DialogContent {
            title: words.install_title().into(),
            body: div().child(words.install_message()).into_any_element(),
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        |shell: &mut Shell, _window, _cx| shell.areas.about.confirm = false,
        cx,
    ))
}

/// A row's value at its right (the version), as Paseo's `aboutValue`.
fn value(text: impl Into<vessel_app_kit::ui::SharedString>, tokens: &Tokens) -> AnyElement {
    let text: vessel_app_kit::ui::SharedString = text.into();
    div()
        .flex_shrink_0()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(text)
        .into_any_element()
}

/// "앱 업데이트": what the updater last did, in Paseo's words, and why
/// it failed; "확인" and the update button at the right.
fn updates_row(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.update();
    let state = &shell.updates.state;
    let status = shell.updates.status(shell.words);
    let checking = matches!(state, UpdateState::Checking);
    let busy = checking || matches!(state, UpdateState::Downloading { .. });
    let ready = matches!(state, UpdateState::Ready { .. });

    let mut check = outline_button(
        "about-check",
        if checking {
            words.checking_short()
        } else {
            words.check()
        },
        !busy,
        tokens,
    );
    if !busy {
        check = check.on_click(cx.listener(|this, _event, _window, cx| this.check_for_updates(cx)));
    }
    // Paseo's `DesktopAppUpdateRow`: the update button is always there,
    // filled, and disabled unless a build is ready to install (the kit's
    // disabled state: half opacity, no hover, no handler); before, an
    // enabled button was faded by hand and still lit on hover.
    let label = shell.updates.action(shell.words);
    let installable = if ready && !busy {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    let mut update = button_with_kind(
        "about-update",
        label,
        ButtonKind::Primary,
        installable,
        tokens,
    )
    .flex_shrink_0();
    if installable == ButtonState::Enabled {
        update = update.on_click(cx.listener(|this, _event, window, cx| {
            this.areas.about.confirm = true;
            this.areas.about.confirm_dialog.focus(window, cx);
            cx.notify();
        }));
    }
    let controls = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(check)
        .child(update)
        .into_any_element();

    // Paseo's `aboutErrorText`: why it failed, under the status.
    let error = shell.updates.problem();
    settings::row_with_error(
        words.app_updates(),
        Some(status),
        error,
        Some(controls),
        false,
        tokens,
    )
}

/// "자동으로 업데이트", with Paseo's switch.
fn automatic_row(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.update();
    let on = shell.updates_automatic();
    let turns = shell.areas.about.automatic_turns;
    let switch = settings::switch("update-automatically", on, true, false, turns, tokens)
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.set_update_automatically(!on, cx);
        }))
        .into_any_element();
    settings::row(
        words.automatic(),
        Some(words.automatic_note().into()),
        Some(switch),
        false,
        tokens,
    )
}

/// "로그 파일": where the app's log is written, and "로그 열기" (track R's
/// `vessel_app_log::dir()`; the folder is made first, so it opens even
/// before the first line is written there).
fn logs_row(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.update();
    // A browser writes no log file (`vessel-app-log` is the desktop's): the
    // row says there is none and its open button is off.
    #[cfg(not(target_arch = "wasm32"))]
    let dir: Option<PathBuf> = vessel_app_log::dir();
    #[cfg(target_arch = "wasm32")]
    let dir: Option<PathBuf> = None;
    let has_dir = dir.is_some();
    let hint: vessel_app_kit::ui::SharedString = dir.as_ref().map_or_else(
        || words.log_unavailable().into(),
        |dir| dir.display().to_string().into(),
    );
    let mut copy = outline_button("about-copy-log-path", words.copy_path(), true, tokens);
    let mut open = outline_button("about-open-logs", words.open_logs(), dir.is_some(), tokens);
    if let Some(dir) = dir {
        let path = dir.display().to_string();
        copy = copy.on_click(cx.listener(move |this, _event, _window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(path.clone()));
            this.notice = Some(this.words.update().log_path_copied().to_owned());
            cx.notify();
        }));
        open = open.on_click(cx.listener(move |_this, _event, _window, cx| {
            if let Err(error) = std::fs::create_dir_all(&dir) {
                tracing::warn!(%error, "the log folder could not be made");
                return;
            }
            cx.open_with_system(&dir);
        }));
    }
    // Paseo's action group: the path's copy only when there is a path.
    let actions = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .when(has_dir, |actions| actions.child(copy))
        .child(open);
    settings::row(
        words.log_file(),
        Some(hint),
        Some(actions.into_any_element()),
        false,
        tokens,
    )
}

/// The tallest the banner's notes are drawn before they scroll, points:
/// about five lines of meta text.
const NOTES_MOST: f32 = 96.0;

/// The note shown once after an update, at the top of the frame.
pub(crate) fn updated(
    shell: &Shell,
    updated: &Updated,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.words.update();
    div()
        .flex()
        .flex_row()
        .items_start()
        .justify_between()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(10.0))
        .bg(tokens.bg_well)
        .border_b_1()
        .border_color(tokens.border_faint)
        // `min_w` 0 on the words: GPUI measures a text's least width
        // unwrapped, so a long line of notes widened the banner past a
        // window that was not maximised and the frame cut it, Dismiss
        // with it (the owner's word 40); now the notes wrap and Dismiss
        // keeps its width.
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .type_(Type::Label)
                        .text_color(tokens.text_primary)
                        .child(shell.updates.banner(shell.words)),
                )
                // At most a few lines tall, scrolled past that: in a small
                // window the whole notes pushed the composer below the
                // frame (About keeps them in full).
                .children(updated.notes.clone().map(|notes| {
                    div()
                        .id("updated-notes")
                        .max_h(px(NOTES_MOST))
                        .overflow_y_scroll()
                        .child(line(notes, tokens.text_secondary))
                })),
        )
        .child(
            div().flex_none().child(
                button("dismiss-updated", words.dismiss(), false, tokens).on_click(cx.listener(
                |this, _event, _window, cx| {
                    this.updates.announcement = None;
                    cx.notify();
                },
            ))),
        )
}

fn line(text: String, color: vessel_app_kit::ui::Hsla) -> Div {
    div().type_(Type::Meta).text_color(color).child(text)
}
