//! The composers beyond the text they hold (track P): a running
//! session's controls, what each agent offers (its lists, kept and
//! refreshed behind the controls), the `/` commands menu over the session's
//! composer and the new chat's, drafts and recall; attachments and `@`
//! files as their slices land. The composer card itself, its
//! pills and the effort card are the draft's (`areas/draft/`), shared by
//! the new chat and a session; this area decides whose options they show
//! and what a choice does there.
//!
//! The rules are `vessel_app_view::composer`'s; this only wires them to
//! the shell, the link and the toasts.

mod attach;
mod capabilities;
mod context;
mod drafts;
mod files;
mod lightbox;
mod local;
mod mentions;
pub(crate) mod modes;
pub(crate) mod offers;
mod pictures;
mod prepared;
mod retention;
pub(crate) mod running;
mod sent;
mod slash;
mod transmission;
mod workspace;

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::time::Instant;

pub(crate) use drafts::keep_drafts;
pub(crate) use files::{attach_button, draft_card, session_card};
pub(crate) use lightbox::overlay;
pub(crate) use local::restore_local;
pub(crate) use prepared::PrepareProblem;
#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "W3 keyed creation request consumer is staged next"
    )
)]
pub(crate) use prepared::WorkspaceInput;
pub(crate) use running::ControlsTarget;
pub(crate) use sent::SentFiles;
pub(crate) use slash::{draft_menu, draft_text, menu, text};
use vessel_app_kit::MenuState;
use vessel_app_kit::ui::App;
use vessel_app_view::composer::drafts::Drafts;
use vessel_app_view::composer::recall::Recall;
use vessel_app_view::composer::running::InFlight;
use vessel_app_view::composer::sent::Pictures as SentPictures;
use vessel_app_view::composer::trays::Trays;
use vessel_app_view::offers::Offers;
use vessel_proto::SessionId;
#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "W3 scoped panel renderer adopts this producer next"
    )
)]
pub(crate) use workspace::workspace_card;

/// The composer's own state.
#[derive(Debug)]
pub(crate) struct ComposerArea {
    /// A running session's option changes waiting for their answers.
    pub(crate) changes: InFlight,
    /// What each agent on each machine offers, as last known.
    pub(crate) offers: Offers,
    /// The `/` menu over the session's composer.
    session_slash: slash::SlashMenu,
    /// The `/` menu over the new chat's composer: its own choice, scroll
    /// and glide, so one composer's menu never moves the other's.
    draft_slash: slash::SlashMenu,
    /// When this area first measured time: the glides' clock.
    epoch: OnceCell<Instant>,
    /// What was typed and not sent, per session and for the new chat.
    pub(crate) drafts: Drafts,
    /// A write of the drafts is waiting for typing to settle.
    pub(crate) saving: bool,
    /// The sent message Up and Down show in the session composer, if one.
    pub(crate) recall: Recall,
    /// Each composer's attached files.
    pub(crate) trays: Trays,
    /// Non-byte context follows the same bounded per-draft ownership.
    pub(crate) contexts: vessel_app_view::composer::trays::DraftTrays<
        vessel_app_view::composer::context::ContextTray,
    >,
    /// Local byte ownership and serialized persistence belong to this composer area.
    local: local::LocalState,
    /// Prompts waiting for their files, per session.
    pub(crate) sends: HashMap<SessionId, attach::Waiting>,
    /// A new chat started for its files, until its session exists (set
    /// while the start is made, which reads the shell only).
    pub(crate) starting: RefCell<Option<attach::Starting>>,
    workspace_starting: std::collections::HashMap<
        (
            vessel_app_view::composer::drafts::DraftKey,
            vessel_app_view::creation::Ticket,
        ),
        attach::Starting,
    >,
    /// The attach button's menu.
    pub(crate) attach_menu: MenuState,
    /// Whether the attach menu is open for the new chat (else the open
    /// session).
    pub(crate) attach_for: Option<vessel_app_view::composer::drafts::DraftKey>,
    /// Pasted and sent images' pictures, decoded.
    pub(crate) pictures: pictures::Pictures,
    /// Sent images' bytes: read back when a message showing them is drawn
    /// (which reads the shell only), or kept from the composer.
    pub(crate) sent: RefCell<SentPictures>,
    lightbox: lightbox::Lightbox,
    session_files: std::rc::Rc<RefCell<mentions::FileMenu>>,
    draft_files: std::rc::Rc<RefCell<mentions::FileMenu>>,
    workspace_files: RefCell<
        vessel_app_view::composer::trays::DraftTrays<std::rc::Rc<RefCell<mentions::FileMenu>>>,
    >,
    mention_root: RefCell<mentions::Root>,
    mention_root_token: RefCell<Option<vessel_app_view::composer::mentions::Mention>>,
    mention_dismissed: RefCell<
        Option<(
            vessel_app_view::composer::drafts::DraftKey,
            vessel_app_view::composer::mentions::Mention,
        )>,
    >,
    mention_root_clock: std::cell::Cell<vessel_app_view::composer::mentions::Ticket>,
    context_preview: context::Preview,
    capabilities: capabilities::Capabilities,
}

impl ComposerArea {
    /// The area as the app starts (a menu needs the app to be made).
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            changes: InFlight::default(),
            offers: Offers::default(),
            session_slash: slash::SlashMenu::default(),
            draft_slash: slash::SlashMenu::default(),
            epoch: OnceCell::new(),
            drafts: Drafts::default(),
            saving: false,
            recall: Recall::default(),
            trays: Trays::default(),
            contexts: vessel_app_view::composer::trays::DraftTrays::default(),
            local: local::LocalState::default(),
            sends: HashMap::new(),
            starting: RefCell::new(None),
            workspace_starting: std::collections::HashMap::new(),
            attach_menu: MenuState::new(cx),
            attach_for: None,
            pictures: pictures::Pictures::default(),
            sent: RefCell::new(SentPictures::default()),
            lightbox: lightbox::Lightbox::new(cx),
            session_files: std::rc::Rc::default(),
            draft_files: std::rc::Rc::default(),
            workspace_files: RefCell::default(),
            mention_root: RefCell::default(),
            mention_root_token: RefCell::default(),
            mention_dismissed: RefCell::default(),
            mention_root_clock: std::cell::Cell::default(),
            context_preview: context::Preview::new(cx),
            capabilities: capabilities::Capabilities::default(),
        }
    }

    /// Seconds since the area first measured time (the glides' clock).
    pub(crate) fn now(&self) -> f32 {
        self.epoch.get_or_init(Instant::now).elapsed().as_secs_f32()
    }
}
