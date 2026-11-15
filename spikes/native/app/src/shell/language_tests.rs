//! Real input controls against an isolated shell, with no link or updater.
//!
//! A replacement typed after changing language checks the retained caret and
//! selection through behavior, without exposing toolkit input internals.

use gpui::{AppContext as _, Render, TestAppContext};
use vessel_app_kit::ui::{Context, Entity, IntoElement, Window};
use vessel_app_kit::{Scheme, SchemeChoice, TextField, Tokens};
use vessel_app_link::Prefs;
use vessel_app_view::localization::{ENGLISH, Locale, Translator};

use super::*;

pub(crate) type Setup = fn(&mut Shell, &mut Window, &mut Context<'_, Shell>);
type Field = fn(&Shell) -> &TextField;

struct Probe {
    shell: Entity<Shell>,
    field: Field,
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let target = self.field;
        self.shell.update(cx, |shell, cx| {
            target(shell).element(&Tokens::of(Scheme::Dark), false, cx)
        })
    }
}

impl Shell {
    pub(crate) fn with_test_link(
        window: &mut Window,
        cx: &mut Context<'_, Self>,
        link: Link,
    ) -> Self {
        let mut shell = Self::without_io(window, cx);
        shell.link = Some(link);
        shell
    }

    pub(crate) fn without_io(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        Self {
            link: None,
            unsent: mpsc::unbounded_channel().0,
            t: vessel_app_view::Lang::En.text(),
            words: Translator::get(ENGLISH).unwrap(),
            language: crate::language::Language::new(),
            fleet: Fleet::default(),
            route: Route::Draft,
            brand: crate::brand::Brand::default(),
            areas: Areas::new(
                crate::areas::settings::PreparedGeneral::new(
                    Translator::get(ENGLISH).unwrap(),
                    &Prefs::default().kept,
                ),
                window,
                cx,
            ),
            prefs: Prefs::default(),
            notice: None,
            notice_detail: None,
            foot_tips: (
                vessel_app_kit::Tooltips::new(),
                std::rc::Rc::new(std::cell::Cell::new(None)),
            ),
            loaded: true,
            arrive_pending: false,
            updates: crate::update_words::UpdateUi::new(Translator::get(ENGLISH).unwrap()),
            sidebar_sheet: None,
        }
    }
}

/// A command queue without a node/runtime task lets area tests supply typed replies.
pub(crate) fn with_commands(
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> (Shell, mpsc::Receiver<Command>) {
    const ROOM: usize = 64;
    let (link, commands) = Link::detached(ROOM).unwrap();
    let mut shell = Shell::without_io(window, cx);
    shell.link = Some(link);
    (shell, commands)
}

pub(crate) fn init(cx: &mut TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
}

/// Uses the actual language-change dispatch and real toolkit key handling.
/// `check` also verifies the area's retained query, step or form state.
pub(crate) fn preserves(cx: &mut TestAppContext, setup: Setup, field: Field, check: fn(&Shell)) {
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            setup(&mut shell, window, cx);
            shell
        });
        Probe { shell, field }
    });
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            field(shell).clear(window, cx);
            field(shell).focus(window, cx);
        });
    });
    cx.run_until_parked();
    cx.simulate_input("typed ABCDE");
    cx.simulate_keystrokes("left left shift-left");
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            assert_eq!(field(shell).text(cx), "typed ABCDE");
            assert!(field(shell).focus_handle(cx).is_focused(window));
            let korean = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
            shell.apply_language(korean, window, cx);
            assert_eq!(shell.words.locale().tag(), "ko");
            assert_eq!(field(shell).text(cx), "typed ABCDE");
            assert!(field(shell).focus_handle(cx).is_focused(window));
            check(shell);
        });
    });
    cx.run_until_parked();
    // The selected C must be replaced in place. Moving/resetting the caret,
    // losing the selection or moving focus yields a different full value.
    cx.simulate_input("X");
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| {
        probe.shell.read_with(cx, |shell, cx| {
            assert_eq!(field(shell).text(cx), "typed ABXDE");
            assert!(field(shell).focus_handle(cx).is_focused(window));
        });
    });
}
