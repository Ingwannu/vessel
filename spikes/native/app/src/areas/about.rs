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
//! - Differences, each for a reason: "새로운 소식" shows the running build's
//!   notes only, and only when the feed gave them (Paseo's changelog lists
//!   every version from its website; Vessel's feed carries one build's
//!   notes, kept by the build that downloaded it); no release channel
//!   (one feed); a switch row for automatic updates, which Paseo does not
//!   have (Vessel's updater has the setting, track R's
//!   `Updater::set_automatic`); "로그 경로가 복사되었습니다." is said in the
//!   frame's foot (the app has no toast yet).

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
    Tokens, Type, TypeExt as _, button, glyph, markdown, outline_button, settings,
};

use super::common::title_bar;
use crate::shell::Shell;
use crate::update::Updated;

/// Paseo's settings page: at most this wide.
const PAGE_WIDTH: f32 = 720.0;

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
}

impl AboutState {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            automatic_turns: 0,
            confirm: false,
            confirm_dialog: DialogState::new(cx),
            notes: false,
            notes_dialog: DialogState::new(cx),
        }
    }
}

/// The About screen.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.update();
    let mut card = settings::card(tokens).child(settings::row(
        words.app_version,
        Some(words.this_device.into()),
        Some(value(words.version(crate::LABEL), tokens)),
        true,
        tokens,
    ));
    if shell.running_notes().is_some() {
        card = card.child(whats_new_row(shell, tokens, cx));
    }
    if shell.updates_on() {
        card = card
            .child(updates_row(shell, tokens, cx))
            .child(automatic_row(shell, tokens, cx));
    } else {
        card = card.child(settings::row(
            words.app_updates,
            Some(words.no_updates.into()),
            None,
            false,
            tokens,
        ));
    }
    card = card.child(logs_row(shell, tokens, cx));
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(title_bar(words.about.to_owned(), None, tokens))
        .child(
            div()
                .id("about-page")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div().w_full().flex().flex_row().justify_center().child(
                        div()
                            .w_full()
                            .max_w(px(PAGE_WIDTH))
                            .px(px(16.0))
                            .pt(px(24.0))
                            .pb(px(16.0))
                            .child(settings::section(words.about, card, tokens)),
                    ),
                ),
        )
        .children(confirm_dialog(shell, tokens, window, cx))
        .children(notes_dialog(shell, tokens, window, cx))
}

/// "새로운 소식": Paseo's row with a chevron, the foreground's on hover.
fn whats_new_row(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.update();
    let neutral = tokens.neutral;
    div().child(
        settings::row(
            words.whats_new,
            Some(words.whats_new_hint.into()),
            Some(glyph(Glyph::ChevronRight, 14.0, neutral.foreground_muted).into_any_element()),
            false,
            tokens,
        )
        .id("whats-new")
        .cursor_pointer()
        .on_click(cx.listener(|this, _event, window, cx| {
            this.areas.about.notes = true;
            this.areas.about.notes_dialog.focus(window, cx);
            cx.notify();
        })),
    )
}

/// The running build's release notes, in a dialog.
fn notes_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let about = &shell.areas.about;
    let notes = shell.running_notes().filter(|_| about.notes)?;
    let words = shell.t.lang.update();
    let body = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .type_(Type::Label)
                .text_color(tokens.neutral.foreground)
                .child(words.version(crate::LABEL)),
        )
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(markdown("release-notes", notes.to_owned())),
        );
    Some(dialog(
        &about.notes_dialog,
        DialogContent {
            title: words.whats_new.into(),
            body: body.into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        |shell: &mut Shell, _window, _cx| shell.areas.about.notes = false,
        cx,
    ))
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
    let words = shell.t.lang.update();
    let footer = vec![
        button("install-cancel", words.cancel, false, tokens)
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.areas.about.confirm = false;
                cx.notify();
            }))
            .into_any_element(),
        button("install-confirm", words.install_confirm, true, tokens)
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.areas.about.confirm = false;
                this.restart_to_update(cx);
            }))
            .into_any_element(),
    ];
    Some(dialog(
        &about.confirm_dialog,
        DialogContent {
            title: words.install_title.into(),
            body: div().child(words.install_message).into_any_element(),
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
fn value(text: String, tokens: &Tokens) -> AnyElement {
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
    let words = shell.t.lang.update();
    let state = &shell.update_state;
    let status = match state {
        UpdateState::Idle => words.status_idle.to_owned(),
        UpdateState::Checking => words.status_checking.to_owned(),
        UpdateState::UpToDate { .. } => words.status_up_to_date.to_owned(),
        UpdateState::Downloading { label, .. } => words.status_downloading(&label.to_string()),
        UpdateState::Ready { label, .. } => words.status_ready(&label.to_string()),
        UpdateState::Failed { .. } => words.status_failed.to_owned(),
    };
    let checking = matches!(state, UpdateState::Checking);
    let busy = checking || matches!(state, UpdateState::Downloading { .. });
    let ready = match state {
        UpdateState::Ready { label, .. } => Some(label.to_string()),
        UpdateState::Idle
        | UpdateState::Checking
        | UpdateState::UpToDate { .. }
        | UpdateState::Downloading { .. }
        | UpdateState::Failed { .. } => None,
    };

    let mut check = outline_button(
        "about-check",
        if checking {
            words.checking_short
        } else {
            words.check
        },
        !busy,
        tokens,
    );
    if !busy {
        check = check.on_click(cx.listener(|this, _event, _window, cx| this.check_for_updates(cx)));
    }
    let mut update = button(
        "about-update",
        ready
            .as_deref()
            .map_or_else(|| words.update.to_owned(), |label| words.update_to(label)),
        true,
        tokens,
    )
    .flex_shrink_0();
    update = if ready.is_some() {
        update.on_click(cx.listener(|this, _event, window, cx| {
            this.areas.about.confirm = true;
            this.areas.about.confirm_dialog.focus(window, cx);
            cx.notify();
        }))
    } else {
        update.opacity(0.5).cursor_default()
    };
    let controls = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(check)
        .child(update)
        .into_any_element();

    // Paseo's `aboutErrorText`: why it failed, under the status.
    let error = match state {
        UpdateState::Failed { error } => Some(error.to_string().into()),
        UpdateState::Idle
        | UpdateState::Checking
        | UpdateState::UpToDate { .. }
        | UpdateState::Downloading { .. }
        | UpdateState::Ready { .. } => None,
    };
    settings::row_with_error(
        words.app_updates,
        Some(status.into()),
        error,
        Some(controls),
        false,
        tokens,
    )
}

/// "자동으로 업데이트", with Paseo's switch.
fn automatic_row(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.update();
    let on = shell.updates_automatic();
    let turns = shell.areas.about.automatic_turns;
    let switch = settings::switch("update-automatically", on, true, false, turns, tokens)
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.set_update_automatically(!on, cx);
        }))
        .into_any_element();
    settings::row(
        words.automatic,
        Some(words.automatic_note.into()),
        Some(switch),
        false,
        tokens,
    )
}

/// "로그 파일": where the app's log is written, and "로그 열기" (track R's
/// `vessel_app_log::dir()`; the folder is made first, so it opens even
/// before the first line is written there).
fn logs_row(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.t.lang.update();
    let dir: Option<PathBuf> = vessel_app_log::dir();
    let has_dir = dir.is_some();
    let hint = dir.as_ref().map_or_else(
        || words.log_unavailable.to_owned(),
        |dir| dir.display().to_string(),
    );
    let mut copy = outline_button("about-copy-log-path", words.copy_path, true, tokens);
    let mut open = outline_button("about-open-logs", words.open_logs, dir.is_some(), tokens);
    if let Some(dir) = dir {
        let path = dir.display().to_string();
        copy = copy.on_click(cx.listener(move |this, _event, _window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(path.clone()));
            this.notice = Some(this.t.lang.update().log_path_copied.to_owned());
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
        words.log_file,
        Some(hint.into()),
        Some(actions.into_any_element()),
        false,
        tokens,
    )
}

/// The note shown once after an update, at the top of the frame.
pub(crate) fn updated(
    shell: &Shell,
    updated: &Updated,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let words = shell.t.lang.update();
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
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .type_(Type::Label)
                        .text_color(tokens.text_primary)
                        .child(words.updated_to(&updated.label)),
                )
                .children(
                    updated
                        .notes
                        .clone()
                        .map(|notes| line(notes, tokens.text_secondary)),
                ),
        )
        .child(
            button("dismiss-updated", words.dismiss, false, tokens).on_click(cx.listener(
                |this, _event, _window, cx| {
                    this.updated = None;
                    cx.notify();
                },
            )),
        )
}

fn line(text: String, color: vessel_app_kit::ui::Hsla) -> Div {
    div().type_(Type::Meta).text_color(color).child(text)
}
