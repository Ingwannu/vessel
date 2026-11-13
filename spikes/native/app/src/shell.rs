//! The window's one view: the route on screen, the fleet as last read,
//! the areas' state, the link's events dispatched to them, and the frame
//! (the title bar, the sidebar on the shell, the page framed beside it:
//! DESIGN.md section 6).

use tokio::sync::mpsc;
use vessel_app_kit::tokens;
use vessel_app_kit::ui::{
    AsyncApp, Context, IntoElement, ParentElement as _, Render, Styled as _, WeakEntity, Window,
    div, px,
};
use vessel_app_link::{Command, Event, Link, Prefs};
use vessel_app_view::{Fleet, Text};

use crate::areas::{self, Areas};
use crate::chrome;
use crate::routes::{self, Route};

/// The window's state.
pub(crate) struct Shell {
    link: Option<Link>,
    /// Every word on screen, in the person's language.
    pub(crate) t: &'static Text,
    pub(crate) fleet: Fleet,
    pub(crate) route: Route,
    pub(crate) areas: Areas,
    /// What the person chose last time (machine, agent, folders).
    pub(crate) prefs: Prefs,
    /// The last thing that happened that belongs to no control, shown in
    /// the frame's foot. Errors about a control are shown beside it.
    pub(crate) notice: Option<String>,
    /// Whether the book has been read once (an empty fleet before then is
    /// not "no machines").
    pub(crate) loaded: bool,
    /// The route was changed where no window was at hand (an event from
    /// the link): what arriving does (focus, placeholders) runs on the
    /// next frame.
    arrive_pending: bool,
    /// The updater, in a build that updates itself (`update.rs`).
    pub(crate) updates: Option<crate::update::Updates>,
    /// What it is doing, as last reported.
    pub(crate) update_state: vessel_app_updater::UpdateState,
    /// The person asked for a check (the About screen then says the
    /// outcome, "up to date" included).
    pub(crate) update_asked: bool,
    /// The note shown once after an update, until dismissed.
    pub(crate) updated: Option<crate::update::Updated>,
}

impl Shell {
    pub(crate) fn new(t: &'static Text, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let areas = Areas::new(window, cx);
        let (link, notice) = match Link::start(crate::LABEL) {
            Ok((link, events)) => {
                cx.spawn(async move |this, cx| receive(this, events, cx).await)
                    .detach();
                (Some(link), None)
            }
            Err(error) => (None, Some(format!("{error}"))),
        };
        let mut shell = Self {
            link,
            t,
            fleet: Fleet::default(),
            route: Route::Draft,
            areas,
            prefs: Prefs::load(),
            notice,
            loaded: false,
            arrive_pending: false,
            updates: None,
            update_state: vessel_app_updater::UpdateState::Idle,
            update_asked: false,
            updated: None,
        };
        crate::update::start(&mut shell, cx);
        shell.announce_update();
        routes::arrive(Route::Draft, &mut shell, window, cx);
        shell
    }

    pub(crate) fn command(&self, command: Command) {
        if let Some(link) = &self.link {
            link.send(command);
        }
    }

    /// Show `route` from where no window is at hand; what arriving does
    /// runs on the next frame.
    pub(crate) fn route_to(&mut self, route: Route) {
        self.route = route;
        self.arrive_pending = true;
    }

    /// Show `route`.
    pub(crate) fn go(&mut self, route: Route, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.route = route;
        routes::arrive(route, self, window, cx);
        cx.notify();
    }

    fn on_event(&mut self, event: Event, cx: &mut Context<'_, Self>) {
        match event {
            Event::Fleet(fleet) => {
                self.fleet = fleet;
                if !self.loaded {
                    self.loaded = true;
                    // With no machine there is nothing to type to: pairing
                    // is the first thing to do.
                    if self.fleet.machines.is_empty() {
                        self.route_to(Route::Pair);
                        self.pairing_opened(cx);
                    }
                }
                self.settle_draft();
                self.retitle();
            }
            Event::Records { session, records } => self.records(&session, &records),
            Event::Started(card) => self.started(&card, cx),
            Event::Sent { .. } => {}
            Event::Paired { node, route } => self.paired(&node, &route, cx),
            Event::PairFailed(why) => self.pair_failed(why),
            Event::Acted(answer) => self.acted(answer, cx),
            Event::Options {
                machine,
                harness,
                outcome,
            } => self.options_arrived(machine, harness, outcome),
            Event::Failed(why) => {
                self.start_failed();
                self.notice = Some(why);
            }
        }
        cx.notify();
    }

    pub(crate) fn reload(&self) {
        self.command(Command::Reload);
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

/// The most events folded in one update of the view.
const BATCH: usize = 512;

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        if std::mem::take(&mut self.arrive_pending) {
            routes::arrive(self.route, self, window, cx);
        }
        let tokens = tokens(cx);
        let frame = routes::render(self.route, self, &tokens, window, cx);
        let content = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(tokens.bg_shell)
            .text_color(tokens.text_primary)
            .child(chrome::title_bar(self, &tokens, window, cx))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_row()
                    .child(areas::sidebar::render(self, &tokens, cx))
                    .child(
                        div().flex_1().h_full().pb(px(8.0)).pr(px(8.0)).child(
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .bg(tokens.bg_base)
                                .border_1()
                                .border_color(tokens.border_faint)
                                .rounded(px(vessel_app_kit::theme::radius::R5))
                                .overflow_hidden()
                                .children(self.updated.as_ref().map(|updated| {
                                    areas::about::updated(self, updated, &tokens, cx)
                                }))
                                .child(frame)
                                .children(
                                    self.notice
                                        .clone()
                                        .map(|notice| areas::common::foot(notice, &tokens)),
                                ),
                        ),
                    ),
            );
        vessel_app_kit::chrome::resizable(content.into_any_element(), window)
    }
}
