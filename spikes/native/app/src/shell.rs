//! The window's one view: the route on screen, the fleet as last read,
//! the areas' state, the link's events dispatched to them, and the frame
//! (the title bar, the sidebar on the shell, the page framed beside it:
//! DESIGN.md section 6).

mod devices;

use tokio::sync::mpsc;
use vessel_app_kit::TypeExt as _;
use vessel_app_kit::frame::{Lines, panel_edges};
use vessel_app_kit::theme::radius::R5;
use vessel_app_kit::theme::spacing::{S1, S2};
use vessel_app_kit::tokens;
use vessel_app_kit::ui::{
    AnyElement, AsyncApp, Context, FluentBuilder as _, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, WeakEntity, Window, div, px,
};
use vessel_app_link::{Command, Event, Link, Prefs};
use vessel_app_view::prefs::Panel;
use vessel_app_view::{Fleet, Text};

use crate::areas::{self, Areas};
use crate::chrome;
use crate::routes::{self, Route};

/// The window's state.
pub(crate) struct Shell {
    link: Option<Link>,
    /// Commands the link gave back, settled on the next update
    /// (`crate::unsent`).
    unsent: mpsc::UnboundedSender<vessel_app_link::Unsent>,
    /// Every word on screen, in the person's language.
    pub(crate) t: &'static Text,
    pub(crate) words: &'static vessel_app_view::localization::Translator,
    pub(crate) language: crate::language::Language,
    pub(crate) fleet: Fleet,
    pub(crate) route: Route,
    pub(crate) areas: Areas,
    /// What the person chose last time (machine, agent, folders).
    pub(crate) prefs: Prefs,
    /// The last thing that happened that belongs to no control, shown in
    /// the frame's foot. Errors about a control are shown beside it.
    pub(crate) notice: Option<String>,
    /// A tooltip's detail for one notice: shown while `notice` is still
    /// that text (any other notice replaces it, so no other caller has to
    /// clear it). Pairing's address is the only one today.
    pub(crate) notice_detail: Option<(String, String)>,
    /// The foot's tooltip and where its line is drawn.
    foot_tips: (vessel_app_kit::Tooltips, vessel_app_kit::tooltip::Place),
    /// Whether the book has been read once (an empty fleet before then is
    /// not "no machines").
    pub(crate) loaded: bool,
    /// The route was changed where no window was at hand (an event from
    /// the link): what arriving does (focus, placeholders) runs on the
    /// next frame.
    arrive_pending: bool,
    /// The updater's facts and prepared translated messages, refreshed together.
    pub(crate) updates: crate::update_words::UpdateUi,
    /// At a phone's width, the sidebar's sheet is open, over what was on
    /// screen when it opened: going anywhere else closes it.
    sidebar_sheet: Option<(Route, Option<vessel_proto::SessionId>)>,
    /// The brand's drawing behind the content (worker B's `brand.rs`).
    pub(crate) brand: crate::brand::Brand,
}
impl Shell {
    pub(crate) fn new(
        prefs: Prefs,
        language: crate::language::Language,
        words: &'static vessel_app_view::localization::Translator,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let general = areas::settings::PreparedGeneral::new(words, &prefs.kept);
        let areas = Areas::new(general, window, cx);
        let (link, notice) = match Link::start(crate::LABEL) {
            Ok((link, events)) => {
                cx.spawn(async move |this, cx| receive(this, events, cx).await)
                    .detach();
                (Some(link), None)
            }
            Err(error) => (None, Some(format!("{error}"))),
        };
        let (unsent, returned) = mpsc::unbounded_channel();
        cx.spawn(async move |this, cx| crate::unsent::receive(this, returned, cx).await)
            .detach();
        let mut shell = Self {
            link,
            unsent,
            t: vessel_app_view::Lang::from_tag(words.locale().tag()).text(),
            words,
            language,
            fleet: Fleet::default(),
            route: Route::Draft,
            brand: crate::brand::Brand::default(),
            areas,
            prefs,
            notice,
            notice_detail: None,
            foot_tips: (
                vessel_app_kit::Tooltips::new(),
                std::rc::Rc::new(std::cell::Cell::new(None)),
            ),
            loaded: false,
            arrive_pending: false,
            updates: crate::update_words::UpdateUi::new(words),
            sidebar_sheet: None,
        };
        areas::desktop::start(&mut shell, window, cx);
        crate::update::start(&mut shell, cx);
        areas::settings::start(&mut shell, window, cx);
        areas::shortcuts::listen(&mut shell, cx);
        shell.announce_update();
        routes::arrive(Route::Draft, &mut shell, window, cx);
        shell
    }

    /// Send `command`; one the link does not take settles on the next
    /// update, after the caller's own code, through the handler its
    /// answer would reach (`crate::unsent`: callers that set what waits
    /// after sending settle too).
    pub(crate) fn command(&self, command: Command) {
        let taken = match areas::desktop::preferences::submit(self, command) {
            Ok(None) => return,
            Err(unsent) => Err(unsent),
            Ok(Some(command)) => match &self.link {
                Some(link) => link.submit(command),
                None => Err(vessel_app_link::Unsent::new(
                    vessel_app_link::Rejected::Closed,
                    command,
                )),
            },
        };
        if let Err(unsent) = taken {
            // The receiver lives as long as the shell's window.
            let _closing = self.unsent.send(unsent);
        }
    }

    /// A command the link did not take: what waits on it settles in the
    /// app's words. Without a link at all, the foot keeps saying why the
    /// link did not start.
    pub(crate) fn unsent(&mut self, unsent: vessel_app_link::Unsent, cx: &mut Context<'_, Self>) {
        areas::desktop::preferences::refused(self, &unsent);
        tracing::warn!(why = %unsent.why, "the link did not take a command");
        let words = self.t.unsent;
        match crate::unsent::settle(unsent, words) {
            crate::unsent::Settle::Ignore => {}
            crate::unsent::Settle::Answer(event) => self.on_event(*event, cx),
            crate::unsent::Settle::Prompt { session, text } => {
                self.prompt_unsent(session, text);
                self.notice = Some(words.to_owned());
            }
            crate::unsent::Settle::Say => {
                if self.link.is_some() {
                    self.notice = Some(words.to_owned());
                }
            }
        }
        cx.notify();
    }

    /// Checked read-only dispatch settles a full or closed command queue.
    pub(crate) fn sync_send(
        &self,
        machine: vessel_app_view::devices::MachineKey,
        ticket: vessel_app_view::devices::Ticket,
    ) -> Result<(), vessel_app_view::sync::Problem> {
        self.link
            .as_ref()
            .ok_or(vessel_app_view::sync::Problem::Offline)?
            .request_sync(machine, ticket)
    }

    /// Show `route` from where no window is at hand; what arriving does
    /// runs on the next frame.
    pub(crate) fn route_to(&mut self, route: Route) {
        self.cancel_session_navigation();
        self.route = route;
        self.arrive_pending = true;
    }

    /// Show `route`.
    pub(crate) fn go(&mut self, route: Route, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.cancel_session_navigation();
        self.route = route;
        routes::arrive(route, self, window, cx);
        cx.notify();
    }

    pub(crate) fn reload(&self) {
        self.command(Command::Reload);
    }

    /// Where the person is: the route and its open session.
    fn place(&self) -> (Route, Option<vessel_proto::SessionId>) {
        let session = self.areas.session.open.as_ref().map(|open| open.session);
        (self.route, session)
    }

    /// Show or hide the sidebar (the title bar's menu button; S2's Mod+B
    /// sends the same preference): at a phone's width its sheet, opened
    /// over the page; wider, the layout's `SidebarPanel` preference, kept
    /// with the settings so it stays as left.
    pub(crate) fn toggle_sidebar(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if matches!(
            areas::sidebar::arrangement(self, window),
            vessel_app_view::panels::SidebarLayout::Sheet
        ) {
            self.sidebar_sheet = match self.sidebar_sheet {
                Some(_) => None,
                None => Some(self.place()),
            };
            cx.notify();
            return;
        }
        let flipped = match self.prefs.settings.layout.sidebar {
            Panel::Shown => Panel::Hidden,
            Panel::Hidden => Panel::Shown,
        };
        self.prefer(
            &vessel_app_view::prefs::PreferenceChange::SidebarPanel(flipped),
            window,
            cx,
        );
    }

    /// Whether the desktop preference requests the sidebar; the typed
    /// layout separately decides whether the current row can show it.
    pub(crate) fn sidebar_shown(&self) -> bool {
        self.prefs.settings.layout.sidebar == Panel::Shown
    }

    /// Whether the sidebar's sheet is open.
    pub(crate) const fn sidebar_open(&self) -> bool {
        self.sidebar_sheet.is_some()
    }
}

/// The link's events, into the view, until either side closes.
///
/// Whatever has queued up by the time one event is taken is folded in the
/// same update, up to [`BATCH`] events: a stream of 20,000 records then
/// costs a few hundred updates of the view, not 20,000.
async fn receive(this: WeakEntity<Shell>, mut events: mpsc::Receiver<Event>, cx: &mut AsyncApp) {
    while let Some(first) = events.recv().await {
        let mut batch = vec![first];
        while batch.len() < BATCH {
            match events.try_recv() {
                Ok(event) => batch.push(event),
                Err(_empty_or_closed) => break,
            }
        }
        let applied = this.update(cx, |shell, cx| {
            for event in batch {
                shell.on_event(event, cx);
            }
        });
        if applied.is_err() {
            return;
        }
    }
}

impl Shell {
    /// The frame's foot with `notice`; its detail, when it has one, as the
    /// line's tooltip (none on touch, as Paseo's tooltips).
    fn foot(&self, notice: String, tokens: &vessel_app_kit::Tokens, touch: bool) -> AnyElement {
        let detail = self
            .notice_detail
            .as_ref()
            .filter(|(of, _)| *of == notice)
            .map(|(_, detail)| detail.clone());
        let Some(detail) = detail else {
            return areas::common::foot(notice, tokens).into_any_element();
        };
        // The tooltip on the words only, so it sits above them, not
        // above the middle of the full-width foot.
        let (tips, place) = &self.foot_tips;
        // Its own width while it fits, wrapping when the window is
        // narrower (it did not shrink and was cut; the owner's word 40).
        let words = div().id("foot").flex_initial().min_w_0().child(notice);
        areas::common::foot(
            div().flex().child(tips.on(words, place, detail, touch)),
            tokens,
        )
        .children(tips.render(tokens))
        .into_any_element()
    }
}

/// The most events folded in one update of the view.
const BATCH: usize = 512;

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let started = crate::region::begin(self, cx);
        if std::mem::take(&mut self.arrive_pending) {
            routes::arrive(self.route, self, window, cx);
        }
        areas::desktop::pending(self, window, cx);
        self.settle_settings(window);
        self.settle_sessions(window, cx);
        self.settle_session_lifecycle(window, cx);
        self.terminal_present(window, cx);
        let tokens = tokens(cx);
        // Sidebar form is independent of global phone chrome. Navigation
        // or docking closes its sheet; the titlebar keeps the global policy.
        let phone = vessel_app_kit::breakpoint::phone(window);
        let sidebar = areas::sidebar::arrangement(self, window);
        let sheet_form = matches!(sidebar, vessel_app_view::panels::SidebarLayout::Sheet);
        let docked = matches!(
            sidebar,
            vessel_app_view::panels::SidebarLayout::Docked { .. }
        );
        if !sheet_form
            || self
                .sidebar_sheet
                .is_some_and(|opened| opened != self.place())
        {
            self.sidebar_sheet = None;
        }
        let touch = areas::draft::compact(window);
        let frame = routes::render(self.route, self, &tokens, window, cx);
        let sheet = self.sidebar_sheet.is_some().then(|| {
            vessel_app_kit::sheet(
                "sidebar-sheet",
                vessel_app_kit::Edge::Left,
                areas::sidebar::render(self, &tokens, true, window, cx).into_any_element(),
                &tokens,
                cx.listener(|this, _event, window, cx| this.toggle_sidebar(window, cx)),
            )
        });
        // The window's face at its root (Geist, Pretendard behind it), so
        // text an area draws without a step of its own still has
        // Pretendard for its Hangul (the owner's font rule; a folded row
        // without a face split "각" into "가" and "ㄱ").
        let content = div()
            .size_full()
            .flex()
            .flex_col()
            .face(vessel_app_kit::Type::Body)
            // Every key-up, for the shortcuts' repeat check (track S's).
            .on_key_up(cx.listener(|this, _event, _window, _cx| this.areas.shortcuts.released()))
            .bg(tokens.bg_shell)
            .text_color(tokens.text_primary)
            .child(chrome::title_bar(self, &tokens, window, cx))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .relative()
                    .flex()
                    .flex_row()
                    .when(docked, |row| {
                        row.child(routes::sidebar(self, &tokens, window, cx))
                    })
                    .child(
                        // `min_w` 0: GPUI measures a text's least width
                        // unwrapped, so without it one long line (a
                        // permission's command) widens the whole area past
                        // the window instead of wrapping.
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .h_full()
                            .pb(px(S2))
                            .pr(px(S2))
                            // Room for the edge's outer line at the top and,
                            // beside the sidebar, at the left (worker B's
                            // two-line edge, the owner's word 36).
                            .when(!phone, |frame| frame.pt(px(S1)))
                            .pl(px(if docked { S1 } else { S2 }))
                            .child(
                                div()
                                    .relative()
                                    .size_full()
                                    .child(
                                        div()
                                            .size_full()
                                            .flex()
                                            .flex_col()
                                            .bg(tokens.neutral.surface0)
                                            .rounded(px(R5))
                                            .overflow_hidden()
                                            .relative()
                                            .child(self.brand.backdrop(&tokens))
                                            .children(self.updates.announcement.as_ref().map(|updated| {
                                                areas::about::updated(self, updated, &tokens, cx)
                                            }))
                                            .child(page::arriving(self, frame))
                                            .children(self.notice.clone().map(|notice| {
                                                self.foot(notice, &tokens, touch)
                                            })),
                                    )
                                    .child(div().absolute().top_0().left_0().size_full().child(
                                        panel_edges(R5, if phone { Lines::One } else { Lines::Two }, &tokens),
                                    )),
                            ),
                    )
                    // Under the title bar, so its menu button stays to close
                    // it with.
                    .children(sheet),
            )
            .children(areas::toasts::render(self, &tokens, window, cx))
            .children(areas::palette::render(self, &tokens, window, cx))
            .children(areas::desktop::render(self, &tokens, window, cx))
            .children(areas::import::render(self, &tokens, window, cx))
            .child(vessel_app_kit::frame_log::drawn(started));
        vessel_app_kit::chrome::resizable(content.into_any_element(), window)
    }
}

mod events;
#[cfg(test)]
pub(crate) mod language_tests;
mod page;
