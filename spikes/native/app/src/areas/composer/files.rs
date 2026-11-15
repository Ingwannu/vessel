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

use std::sync::Arc;

use gpui::{ExternalPaths, Image, ImageFormat, ImageSource, PathPromptOptions};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::attachment::{
    Body, PillClick, PillHandlers, Standing, attachment_pill, attachment_tray,
};
use vessel_app_kit::breakpoint::TOUCH_TARGET;
use vessel_app_kit::field::composer_hooks::{KeyAnswer, Pasted};
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, MouseButton,
    MouseDownEvent, ParentElement as _, SharedString, Styled as _, Window, div, px,
};
use vessel_app_kit::{MenuHandlers, MenuItem, Tokens, composer_card, menu};
use vessel_app_view::composer::attach::Refusal;
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer::tray::{Again, Item, ItemState, Origin, Problem};
use vessel_proto::AttachmentKind;

use super::attach::Incoming;
use crate::shell::Shell;

/// The session composer's card: its `/` menu, its files, its text.
pub(crate) fn session_card(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let menu = super::menu(shell, tokens, window, cx);
    let files = shell
        .session_tray_key()
        .and_then(|key| tray(shell, key, tokens, touch, cx));
    let attach = (shell.areas.composer.attach_for == Some(false))
        .then(|| attach_menu(shell, tokens, touch, cx))
        .flatten();
    dropped_on(composer_card(tokens, touch), false, tokens, cx)
        .relative()
        .children(menu)
        .children(attach)
        .children(files)
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
    let menu = super::draft_menu(shell, tokens, window, cx);
    let files = tray(shell, DraftKey::New, tokens, touch, cx);
    let attach = (shell.areas.composer.attach_for == Some(true))
        .then(|| attach_menu(shell, tokens, touch, cx))
        .flatten();
    dropped_on(composer_card(tokens, touch), true, tokens, cx)
        .relative()
        .children(menu)
        .children(attach)
        .children(files)
        .child(super::draft_text(shell, tokens, touch, cx))
}

/// Files dropped on `card` join its composer; the card's edge takes the
/// accent while files are dragged over it.
fn dropped_on(card: Div, new_chat: bool, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let accent = tokens.neutral.accent;
    card.drag_over::<ExternalPaths>(move |style, _paths, _window, _cx| style.border_color(accent))
        .on_drop(
            cx.listener(move |shell, paths: &ExternalPaths, _window, cx| {
                let files = paths
                    .paths()
                    .iter()
                    .cloned()
                    .filter_map(Incoming::file)
                    .collect();
                shell.composer_files_for(new_chat, files, cx);
            }),
        )
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
                shell.areas.composer.attach_for = Some(new_chat);
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

/// The attach menu, when open (Paseo's rows: an image, a file; the forge's
/// issues and the plugins' rows are not Vessel's yet).
fn attach_menu(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let words = shell.words.composer();
    let items = [
        MenuItem::new(words.attach_image().to_owned()),
        MenuItem::new(words.attach_file().to_owned()),
    ];
    let handlers = MenuHandlers {
        state: |shell: &mut Shell| &mut shell.areas.composer.attach_menu,
        chose: std::rc::Rc::new(|shell: &mut Shell, index, _window, cx| {
            shell.composer_pick(index == 0, cx);
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

impl Shell {
    /// Files for the new chat (`new_chat`) or the open session.
    fn composer_files_for(
        &mut self,
        new_chat: bool,
        files: Vec<Incoming>,
        cx: &mut Context<'_, Self>,
    ) {
        let key = if new_chat {
            Some(DraftKey::New)
        } else {
            self.session_tray_key()
        };
        if let Some(key) = key
            && !files.is_empty()
        {
            self.composer_add(key, files, cx);
        }
    }

    /// Images or files pasted into a composer: attached (the text keeps a
    /// text paste).
    pub(crate) fn composer_pasted(
        &mut self,
        new_chat: bool,
        pasted: Vec<Pasted>,
        cx: &mut Context<'_, Self>,
    ) -> KeyAnswer {
        let files: Vec<Incoming> = pasted
            .into_iter()
            .flat_map(|paste| match paste {
                Pasted::Image { mime, bytes } => vec![Incoming::pasted(mime, bytes)],
                Pasted::Files(paths) => paths.into_iter().filter_map(Incoming::file).collect(),
            })
            .collect();
        if files.is_empty() {
            return KeyAnswer::Pass;
        }
        self.composer_files_for(new_chat, files, cx);
        KeyAnswer::Taken
    }

    /// The system's file dialog for the menu's row (an image or a file:
    /// GPUI's dialog takes no type filter, so both pick any file).
    fn composer_pick(&mut self, image: bool, cx: &mut Context<'_, Self>) {
        let new_chat = self.areas.composer.attach_for.take().unwrap_or(false);
        let words = self.words.composer();
        let prompt = if image {
            words.attach_image()
        } else {
            words.attach_file()
        };
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(SharedString::from(prompt.to_owned())),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let files: Vec<Incoming> = paths.into_iter().filter_map(Incoming::file).collect();
            drop(this.update(cx, |shell, cx| {
                shell.composer_files_for(new_chat, files, cx);
            }));
        })
        .detach();
    }

    // The kit's pill takes a plain function per click; one per composer.
    fn session_pill_removed(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(key) = self.session_tray_key() {
            self.composer_remove(key, index, cx);
        }
    }

    fn draft_pill_removed(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.composer_remove(DraftKey::New, index, cx);
    }

    fn session_pill_retried(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(DraftKey::Session { session }) = self.session_tray_key() {
            let tray = self
                .areas
                .composer
                .trays
                .tray(DraftKey::Session { session });
            if let Some(key) = tray.items().get(index).map(|item| item.key) {
                let _retried = tray.retry(key);
            }
            self.composer_pump(session, cx);
        }
    }

    fn composer_remove(&mut self, key: DraftKey, index: usize, cx: &mut Context<'_, Self>) {
        let tray = self.areas.composer.trays.tray(key);
        if let Some(item) = tray.items().get(index).map(|item| item.key) {
            let _removed = tray.remove(item);
        }
        match key {
            DraftKey::Session { session } => self.composer_pump(session, cx),
            DraftKey::New => cx.notify(),
        }
    }
}

/// `key`'s tray, when it holds files.
fn tray(
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
    let new_chat = key == DraftKey::New;
    let id = if new_chat {
        "draft-file"
    } else {
        "session-file"
    };
    let retried: PillClick<Shell> = Shell::session_pill_retried;
    let removed: PillClick<Shell> = if new_chat {
        Shell::draft_pill_removed
    } else {
        Shell::session_pill_removed
    };
    let pills = tray.items().iter().enumerate().map(|(index, item)| {
        let retry = matches!(
            item.state,
            ItemState::Failed {
                again: Again::Resume(_) | Again::Restart,
                ..
            }
        ) && !new_chat;
        let handlers = PillHandlers {
            on_open: retry.then_some(retried),
            on_remove: removed,
        };
        let body = body(shell, item);
        attachment_pill(
            id,
            index,
            &body,
            standing(item),
            &handlers,
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
    let words = shell.words.composer();
    let subtitle = trouble.unwrap_or_else(|| {
        item.name.as_str().rsplit_once('.').map_or_else(
            || words.file().to_owned(),
            |(_, extension)| extension.to_uppercase(),
        )
    });
    Body::Label {
        icon: Glyph::FileText,
        title: SharedString::from(item.name.as_str().to_owned()),
        subtitle: SharedString::from(subtitle),
    }
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

/// An image's picture: a file on this device read by GPUI's image cache, a
/// pasted one decoded once and kept while a tray holds it.
fn picture(
    shell: &Shell,
    origin: &Origin,
    mime: &vessel_proto::conversation::MimeType,
) -> ImageSource {
    match origin {
        Origin::File(path) => ImageSource::from(path.clone()),
        Origin::Pasted(bytes) => shell.areas.composer.pictures.of(bytes, mime.as_str()),
    }
}

/// Pasted images' pictures, kept while a tray holds their bytes.
#[derive(Default)]
pub(crate) struct Pictures(std::cell::RefCell<Vec<(Arc<[u8]>, ImageSource)>>);

impl std::fmt::Debug for Pictures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pictures").finish_non_exhaustive()
    }
}

impl Pictures {
    fn of(&self, bytes: &Arc<[u8]>, mime: &str) -> ImageSource {
        let mut kept = self.0.borrow_mut();
        // A picture whose bytes no tray holds any more goes.
        kept.retain(|(held, _)| Arc::strong_count(held) > 1);
        if let Some((_, source)) = kept.iter().find(|(held, _)| Arc::ptr_eq(held, bytes)) {
            return source.clone();
        }
        let format = match mime {
            "image/jpeg" => ImageFormat::Jpeg,
            "image/gif" => ImageFormat::Gif,
            "image/webp" => ImageFormat::Webp,
            _ => ImageFormat::Png,
        };
        let image = Arc::new(Image::from_bytes(format, bytes.to_vec()));
        let source = ImageSource::from(image);
        kept.push((Arc::clone(bytes), source.clone()));
        source
    }
}
