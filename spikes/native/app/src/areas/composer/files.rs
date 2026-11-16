//! The composers' files on screen (track P's row c): each composer's card
//! with its `/` menu, its tray of pills over the text (Paseo's
//! `renderAttachmentTray`), files dropped on it, pasted into it or picked
//! from the attach button's menu (Paseo's `+` and its `AttachmentMenuList`:
//! an image, a file).
//!
//! A file on its way shows its progress along its pill's foot; one refused
//! or failed shows why in its pill's second line, and a click on a failed
//! one tries it again (Paseo sends a file with its message in one call and
//! has neither).

use gpui::ExternalPaths;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::attachment::{Body, Standing, attachment_pill_scoped, attachment_tray};
use vessel_app_kit::breakpoint::TOUCH_TARGET;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, MouseButton,
    MouseDownEvent, ParentElement as _, SharedString, Styled as _, Window, div, px,
};
use vessel_app_kit::{MenuHandlers, MenuItem, Tokens, composer_card, menu};
use vessel_app_view::composer::attach::Refusal;
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer::tray::{Item, ItemState, Problem};
use vessel_proto::AttachmentKind;

mod intents;
mod select;

use super::pictures::picture;
use crate::shell::Shell;

/// The session composer's card: its `/` menu, its files, its text.
pub(crate) fn session_card(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let menu = shell
        .session_tray_key()
        .and_then(|target| super::mentions::menu(shell, target, tokens, window, cx))
        .or_else(|| super::menu(shell, tokens, window, cx));
    let files = shell
        .session_tray_key()
        .and_then(|key| tray(shell, key, tokens, touch, cx));
    let attach = (shell.areas.composer.attach_for.is_some()
        && shell.areas.composer.attach_for == shell.session_tray_key())
    .then(|| attach_menu(shell, tokens, touch, cx))
    .flatten();
    dropped_on(
        composer_card(tokens, touch),
        shell.session_tray_key(),
        tokens,
        cx,
    )
    .relative()
    .children(menu)
    .children(attach)
    .children(files)
    .children(
        shell
            .session_tray_key()
            .and_then(|target| super::local::notice(shell, target, tokens)),
    )
    .children(
        shell
            .session_tray_key()
            .and_then(|target| super::transmission::notice(shell, target, tokens, cx)),
    )
    .children(
        shell
            .session_tray_key()
            .and_then(|target| super::context::tray(shell, target, tokens, touch, cx)),
    )
    .child(super::text(shell, tokens, touch, cx))
}

/// The new chat's card: its `/` menu, its files, its text.
pub(crate) fn draft_card(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let menu = super::mentions::menu(shell, DraftKey::New, tokens, window, cx)
        .or_else(|| super::draft_menu(shell, tokens, window, cx));
    let files = tray(shell, DraftKey::New, tokens, touch, cx);
    let attach = (shell.areas.composer.attach_for == Some(DraftKey::New))
        .then(|| attach_menu(shell, tokens, touch, cx))
        .flatten();
    dropped_on(
        composer_card(tokens, touch),
        Some(DraftKey::New),
        tokens,
        cx,
    )
    .relative()
    .children(menu)
    .children(attach)
    .children(files)
    .children(super::local::notice(shell, DraftKey::New, tokens))
    .children(super::context::tray(
        shell,
        DraftKey::New,
        tokens,
        touch,
        cx,
    ))
    .child(super::draft_text(shell, tokens, touch, cx))
}

/// Files dropped on `card` join its composer; the card's edge takes the
/// accent while files are dragged over it.
pub(super) fn dropped_on(
    card: Div,
    target: Option<DraftKey>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    use vessel_app_view::composer::context::{ContextAttachment, WorkspaceFile};
    let accent = tokens.neutral.accent;
    card.drag_over::<ExternalPaths>(move |style, _paths, _window, _cx| style.border_color(accent))
        .drag_over::<WorkspaceFile>(move |style, _file, _window, _cx| style.border_color(accent))
        .on_drop(
            cx.listener(move |shell, paths: &ExternalPaths, _window, cx| {
                if let Some(target) = target {
                    shell.composer_paths_into(target, paths.paths().to_vec(), cx);
                }
            }),
        )
        .on_drop(cx.listener(move |shell, file: &WorkspaceFile, window, cx| {
            let Some(target) = target else {
                return;
            };
            let item = ContextAttachment::WorkspaceFile(file.clone());
            if let Err(problem) =
                shell.composer_attach_context(target, &file.scope, item, window, cx)
            {
                shell.composer_context_refused(problem, cx);
            }
        }))
}

/// The attach button across (Paseo's `TOOLBAR_CONTROL_SIZES.pointer`;
/// the spacing scale's 28; the touch target on touch).
const CONTROL: f32 = spacing::S7;

/// Its glyph (Paseo's `ICON_SIZE.md`).
const CONTROL_GLYPH: f32 = 16.0;

/// The room under the tray, above the text (Vessel's own: the pills' top
/// left remove buttons need the room above, the text the room below).
const TRAY_GAP: f32 = spacing::S1;

/// The attach button (Paseo's `+`), opening its menu where it was
/// pressed.
pub(crate) fn attach_button(
    new_chat: bool,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    attach_button_with(None, new_chat, touch, tokens, cx)
}

pub(super) fn attach_button_at(
    target: DraftKey,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    attach_button_with(Some(target), false, touch, tokens, cx)
}

fn attach_button_with(
    target: Option<DraftKey>,
    new_chat: bool,
    touch: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let size = if touch { TOUCH_TARGET } else { CONTROL };
    div()
        .id(if new_chat {
            "draft-attach"
        } else {
            "session-attach"
        })
        .size(px(size))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .text_color(neutral.foreground_muted)
        .hover(|style| style.bg(neutral.interaction_highlight))
        .child(glyph(Glyph::Plus, CONTROL_GLYPH, neutral.foreground_muted))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |shell, event: &MouseDownEvent, window, cx| {
                let target = target.or_else(|| {
                    if new_chat {
                        Some(DraftKey::New)
                    } else {
                        shell.session_tray_key()
                    }
                });
                let Some(target) = target.filter(|target| shell.composer_field(*target).is_some())
                else {
                    return;
                };
                shell.areas.composer.attach_for = Some(target);
                shell
                    .areas
                    .composer
                    .attach_menu
                    .open(event.position, window, cx);
                cx.notify();
            }),
        )
        .into_any_element()
}

#[cfg(not(target_arch = "wasm32"))]
const CHOICES: &[select::Acquire] = &[
    select::Acquire::Image,
    select::Acquire::ClipboardImage,
    select::Acquire::File,
];
#[cfg(target_arch = "wasm32")]
const CHOICES: &[select::Acquire] = &[select::Acquire::Image, select::Acquire::File];

/// The attach menu, when open (Paseo's rows: an image, a file; the forge's
/// issues and the plugins' rows are not Vessel's yet).
pub(super) fn attach_menu(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let words = shell.words.composer();
    let items: Vec<_> = CHOICES
        .iter()
        .map(|choice| {
            MenuItem::new(
                match choice {
                    select::Acquire::Image => words.attach_image(),
                    select::Acquire::ClipboardImage => words.paste_image(),
                    select::Acquire::File => words.attach_file(),
                }
                .to_owned(),
            )
        })
        .collect();
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.composer.attach_menu,
        chose: std::rc::Rc::new(|shell: &mut Shell, index, window, cx| {
            if let Some(choice) = CHOICES.get(index) {
                shell.composer_choose(*choice, window, cx);
            }
        }),
    };
    menu(
        "composer-attach-menu",
        &shell.areas.composer.attach_menu,
        &items,
        &handlers,
        tokens,
        touch,
        cx,
    )
}

/// `key`'s tray, when it holds files.
pub(super) fn tray(
    shell: &Shell,
    key: DraftKey,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let tray = shell.areas.composer.trays.get(key)?;
    if tray.is_empty() {
        return None;
    }
    let pills = tray.items().iter().enumerate().map(|(index, item)| {
        let (open, remove) = intents::callbacks(key, item);
        let body = body(shell, item);
        attachment_pill_scoped(
            "composer-file",
            index,
            &body,
            standing(item),
            open,
            remove,
            touch,
            tokens,
            cx,
        )
    });
    Some(
        attachment_tray()
            .pb(px(TRAY_GAP))
            .children(pills)
            .into_any_element(),
    )
}

/// Where a file stands, for its pill.
fn standing(item: &Item) -> Standing {
    match &item.state {
        ItemState::Uploading(received) => {
            // Thousandths, then a share, without a cast or a float operator.
            let permille = received
                .saturating_mul(1000)
                .checked_div(item.size.max(1))
                .unwrap_or(0)
                .min(1000);
            let permille = f32::from(u16::try_from(permille).unwrap_or(1000));
            Standing::Sending(permille.mul_add(0.001, 0.0))
        }
        ItemState::Waiting(_) => Standing::Sending(0.0),
        ItemState::Refused(_) | ItemState::Failed { .. } => Standing::Trouble,
        ItemState::Held | ItemState::Attached | ItemState::Uploaded(_) => Standing::Quiet,
    }
}

/// A file's pill body: an image's picture, any other file (and any file in
/// trouble, so the reason shows) as its label.
fn body(shell: &Shell, item: &Item) -> Body {
    let trouble = trouble(shell, &item.state);
    if item.kind == AttachmentKind::Image && trouble.is_none() {
        return Body::Image(Some(picture(shell, &item.origin, &item.mime)));
    }
    let subtitle = trouble.unwrap_or_else(|| kind_line(shell, item.name.as_str()));
    Body::Label {
        icon: Glyph::FileText,
        title: SharedString::from(item.name.as_str().to_owned()),
        subtitle: SharedString::from(subtitle),
    }
}

/// A file's second line: its extension, or the word for a file (Paseo's
/// file pill).
pub(super) fn kind_line(shell: &Shell, name: &str) -> String {
    name.rsplit_once('.').map_or_else(
        || shell.words.composer().file().to_owned(),
        |(_, extension)| extension.to_uppercase(),
    )
}

/// Why a file cannot go, in the person's words.
fn trouble(shell: &Shell, state: &ItemState) -> Option<String> {
    let words = shell.words.composer();
    let line = match state {
        ItemState::Refused(Refusal::Agent(AttachmentKind::Image)) => words.file_no_images(),
        ItemState::Refused(Refusal::Agent(AttachmentKind::Audio)) => words.file_no_audio(),
        ItemState::Refused(Refusal::Agent(AttachmentKind::File) | Refusal::TooLarge { .. }) => {
            words.file_too_large()
        }
        ItemState::Refused(Refusal::Empty) => words.file_empty(),
        ItemState::Refused(Refusal::TooMany { .. }) => words.file_too_many(),
        ItemState::Refused(Refusal::InlineTotal { .. }) => words.file_inline_total(),
        ItemState::Failed { problem, .. } => match problem {
            Problem::NotItsType => words.file_not_its_type(),
            Problem::Changed => words.file_changed(),
            Problem::Unreadable(_) => words.file_unreadable(),
            Problem::Node(_) => words.file_failed(),
        },
        ItemState::Held
        | ItemState::Attached
        | ItemState::Waiting(_)
        | ItemState::Uploading(_)
        | ItemState::Uploaded(_) => return None,
    };
    Some(line.to_owned())
}
