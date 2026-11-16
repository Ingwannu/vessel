//! Actual GPUI events through the production listener, without a link.
//! The terminal owner query has a separate actual focus/route/close fixture.

use crate::shell::{Shell, language_tests};
use gpui::{
    AppContext as _, FocusHandle, InputPreference, Render, TestAppContext, VisualTestContext,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vessel_app_kit::ui::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::prefs::Theme;
use vessel_app_view::shortcuts::Platform;

struct Probe {
    shell: Entity<Shell>,
    focus: FocusHandle,
    raw: Rc<Cell<usize>>,
    overlay: vessel_app_kit::menu::MenuState,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let raw = Rc::clone(&self.raw);
        let tokens = Tokens::of(Scheme::Dark);
        let (field, palette) = self.shell.update(cx, |shell, cx| {
            let field = shell.areas.draft.composer.element(&tokens, false, cx);
            let palette = crate::areas::palette::render(shell, &tokens, window, cx);
            (field, palette)
        });
        let menu = vessel_app_kit::menu::menu(
            "shortcut-test-overlay",
            &self.overlay,
            &[vessel_app_kit::menu::MenuItem::new("Local menu")],
            &vessel_app_kit::menu::MenuHandlers {
                state: |probe: &mut Probe| &mut probe.overlay,
                chose: Rc::new(|_, _, _, _| {}),
            },
            &tokens,
            false,
            cx,
        );
        div()
            .size_full()
            .key_context("ShortcutProbe")
            .track_focus(&self.focus)
            .capture_key_down(move |_, _, _| raw.set(raw.get().saturating_add(1)))
            .child(field)
            .children(palette)
            .children(menu)
    }
}
fn setup(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    language_tests::init(cx);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            super::listen(&mut shell, window, cx);
            shell
        });
        Probe {
            shell,
            focus: cx.focus_handle(),
            raw: Rc::default(),
            overlay: vessel_app_kit::menu::MenuState::new(cx),
        }
    });
    view.update_in(cx, |probe, window, cx| probe.focus.focus(window, cx));
    cx.run_until_parked();
    (view, cx)
}
fn chord() -> &'static str {
    match Platform::native() {
        Platform::Mac => "cmd-alt-t",
        Platform::Other => "ctrl-alt-t",
    }
}

#[gpui::test]
fn real_key_events_advance_preferences_through_existing_dispatch(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    for expected in [
        Theme::Zinc,
        Theme::Midnight,
        Theme::Claude,
        Theme::Ghostty,
        Theme::PureBlack,
        Theme::Light,
        Theme::Dark,
        Theme::System,
    ] {
        cx.simulate_keystrokes(chord());
        cx.run_until_parked();
        view.read_with(cx, |probe, cx| {
            assert_eq!(
                probe.shell.read(cx).prefs.settings.appearance.theme,
                expected
            );
        });
    }
}

#[gpui::test]
fn platform_base_key_fallback_survives_non_latin_typed_text(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    cx.simulate_keystrokes(match Platform::native() {
        Platform::Mac => "cmd-alt-t->ㅅ",
        Platform::Other => "ctrl-alt-t->ㅅ",
    });
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        assert_eq!(
            probe.shell.read(cx).prefs.settings.appearance.theme,
            Theme::System.next()
        );
    });
}

#[gpui::test]
fn real_key_events_leave_theme_alone_while_palette_is_open(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    view.update_in(cx, |probe, window, cx| {
        probe
            .shell
            .update(cx, |shell, cx| shell.open_palette(window, cx));
    });
    cx.run_until_parked();
    cx.simulate_keystrokes(chord());
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        assert!(probe.shell.read(cx).areas.palette.is_open());
        assert_eq!(
            probe.shell.read(cx).prefs.settings.appearance.theme,
            Theme::System
        );
    });
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        assert!(!probe.shell.read(cx).areas.palette.is_open());
    });
    cx.simulate_keystrokes(chord());
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        assert_eq!(
            probe.shell.read(cx).prefs.settings.appearance.theme,
            Theme::Zinc
        );
    });
}

#[gpui::test]
fn character_preferred_platform_event_propagates_and_clears_pending(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    view.update_in(cx, |probe, _window, cx| {
        probe.shell.update(cx, |shell, _| {
            let action = "go-board".parse().unwrap();
            let keys =
                vessel_app_view::prefs::Bindings::try_from(vec!["G S".parse().unwrap()]).unwrap();
            drop(shell.prefs.settings.shortcuts.insert(action, keys));
        });
    });
    cx.simulate_keystrokes("g");
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            assert!(shell.areas.shortcuts.engine.is_pending());
            shell.areas.draft.composer.focus(window, cx);
        });
    });
    cx.run_until_parked();
    let observed = Rc::new(RefCell::new(None));
    let _observer = cx.update({
        let observed = Rc::clone(&observed);
        move |_, cx| {
            cx.intercept_keystrokes(move |event, _, _| {
                *observed.borrow_mut() = Some(event.input_preference);
            })
        }
    });
    let before = view.read_with(cx, |probe, _| probe.raw.get());
    cx.update(|window, cx| {
        let result = window.dispatch_event(
            gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke: gpui::Keystroke::parse(chord()).unwrap(),
                is_held: false,
                prefer_character_input: true,
            }),
            cx,
        );
        assert!(result.propagate, "the app consumed text-preferred input");
    });
    cx.run_until_parked();
    assert_eq!(*observed.borrow(), Some(InputPreference::CharacterInput));
    view.read_with(cx, |probe, cx| {
        assert!(
            probe.raw.get() > before,
            "raw key handlers never received the event"
        );
        let shell = probe.shell.read(cx);
        assert_eq!(shell.prefs.settings.appearance.theme, Theme::System);
        assert!(!shell.areas.shortcuts.engine.is_pending());
    });
}

#[gpui::test]
fn hidden_window_key_events_neither_dispatch_nor_continue_a_prefix(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let before = view.read_with(cx, |probe, cx| {
        probe.shell.read(cx).prefs.settings.appearance.theme
    });
    cx.simulate_visibility_change(gpui::WindowVisibility::Hidden);
    cx.run_until_parked();
    cx.simulate_keystrokes(chord());
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        let shell = probe.shell.read(cx);
        assert_eq!(shell.prefs.settings.appearance.theme, before);
        assert!(!shell.areas.shortcuts.engine.is_pending());
    });
    cx.simulate_visibility_change(gpui::WindowVisibility::Visible);
    cx.run_until_parked();
    cx.simulate_keystrokes(chord());
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        assert_eq!(
            probe.shell.read(cx).prefs.settings.appearance.theme,
            before.next()
        );
    });
}

#[gpui::test]
fn actual_menu_context_pauses_settings_capture_before_recording(cx: &mut TestAppContext) {
    use vessel_app_view::settings::Section;
    use vessel_app_view::shortcuts::sequence_capture::{CaptureRole, SequenceCapture};
    use vessel_app_view::shortcuts::{Action, binding_id, default_context};
    let (view, cx) = setup(cx);
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.open_settings(Some(Section::Shortcuts), window, cx);
            shell.areas.shortcuts.page.capture = Some(SequenceCapture::new(
                binding_id(Action::Palette, default_context(Platform::native())).unwrap(),
                CaptureRole::Ordinary,
            ));
            assert!(!shell.dispatch_shortcut_command(Action::CycleTheme, window, cx));
        });
        probe
            .overlay
            .open(gpui::point(px(20.0), px(20.0)), window, cx);
    });
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let capture = shell.areas.shortcuts.page.capture.take();
            let theme = shell.prefs.settings.appearance.theme;
            assert!(!shell.dispatch_shortcut_command(Action::CycleTheme, window, cx));
            assert_eq!(shell.prefs.settings.appearance.theme, theme);
            shell.areas.shortcuts.page.capture = capture;
        });
    });
    let key = match Platform::native() {
        Platform::Mac => "cmd-k",
        Platform::Other => "ctrl-k",
    };
    cx.simulate_keystrokes(key);
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        assert_eq!(
            probe
                .shell
                .read(cx)
                .areas
                .shortcuts
                .page
                .capture
                .as_ref()
                .unwrap()
                .chords()
                .count(),
            0
        );
    });
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    cx.simulate_keystrokes(key);
    cx.run_until_parked();
    view.read_with(cx, |probe, cx| {
        let shell = probe.shell.read(cx);
        assert_eq!(
            shell
                .areas
                .shortcuts
                .page
                .capture
                .as_ref()
                .unwrap()
                .chords()
                .count(),
            1
        );
        assert!(!shell.areas.palette.is_open());
    });
}
