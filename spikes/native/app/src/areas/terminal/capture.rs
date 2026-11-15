//! Linux display-test fixture for held reads and typed history failures.
//! The normal link follows a private fake terminal; only its history
//! receipt is held/refused here. This proves rendering, not a network failure.
//! No override or fake reply is compiled into the installed application.

use crate::{areas::session::Open, language::Language, shell::Shell};
use gpui::{AppContext as _, Bounds, px, size};
use std::path::PathBuf;
use vessel_app_link::{ActAnswer, ActFailed, ActKind, Prefs, TerminalOpening};
use vessel_proto::SessionId;

#[derive(Clone, Copy)]
enum State {
    Ready,
    Loading,
    Unread,
}

pub(crate) fn run() {
    let config = PathBuf::from(std::env::var_os("VESSEL_CONFIG_DIR").unwrap());
    let fleet = config.parent().unwrap();
    assert!(
        fleet.join("wire-gap-shapes.txt").is_file(),
        "a verified disposable fleet is required"
    );
    let nodes = std::fs::read_to_string(fleet.join("nodes")).unwrap();
    for line in nodes.lines() {
        let addr: std::net::SocketAddr = line.split_whitespace().nth(1).unwrap().parse().unwrap();
        assert!(addr.ip().is_loopback());
    }
    let session: SessionId = std::env::var("VESSEL_TEST_TERMINAL_SESSION")
        .unwrap()
        .parse()
        .unwrap();
    let state = match std::env::var("VESSEL_TEST_TERMINAL_STATE").as_deref() {
        Ok("ready") => State::Ready,
        Ok("loading") => State::Loading,
        Ok("unread") => State::Unread,
        _ => panic!("choose a terminal capture state"),
    };
    let prefs = Prefs::load();
    let language = Language::new();
    let words = language.prepare(prefs.settings.language).unwrap();
    let (width, height) = crate::window_size();
    gpui_platform::application()
        .with_assets(vessel_app_kit::Assets)
        .run(move |cx| {
            vessel_app_kit::init(cx, crate::scheme_choice()).unwrap();
            crate::motion::follow(cx);
            crate::motion::animations(prefs.settings.appearance.animations.on(), cx);
            cx.on_window_closed(|cx, _| cx.quit()).detach();
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
            let options =
                vessel_app_kit::chrome::window_options("Vessel terminal fixture".into(), bounds);
            let _window = cx
                .open_window(options, |window, cx| {
                    let shell = cx.new(|cx| Shell::new(prefs, language, words, window, cx));
                    let handle = window.window_handle();
                    let mut opened = false;
                    cx.observe(&shell, move |shell, cx| {
                        if opened || !shell.read(cx).loaded {
                            return;
                        }
                        opened = true;
                        cx.update_window(handle, |_, window, cx| {
                            shell.update(cx, |shell, cx| {
                                open(shell, session, state, window, cx);
                            });
                        })
                        .unwrap();
                    })
                    .detach();
                    vessel_app_kit::root(shell, window, cx)
                })
                .unwrap();
            cx.activate(true);
        });
}

fn open(
    shell: &mut Shell,
    session: SessionId,
    state: State,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<'_, Shell>,
) {
    shell.open_session(
        Open {
            machine: "laptop".into(),
            session,
            title: "Terminal / 터미널".into(),
            agent: "fake".into(),
            structured: false,
            live: true,
        },
        window,
        cx,
    );
    match state {
        State::Ready => {}
        State::Loading => {
            // Refuse the old read's completion while live records remain held.
            shell.areas.terminal.shown =
                Some(TerminalOpening::new("laptop".parse().unwrap(), session));
        }
        State::Unread => {
            let terminal = shell.areas.terminal.shown.clone();
            shell.acted(
                ActAnswer {
                    machine: "laptop".into(),
                    session,
                    terminal,
                    kind: ActKind::Log,
                    result: Err(ActFailed {
                        code: None,
                        message: "fixture history refusal".into(),
                    }),
                },
                cx,
            );
        }
    }
    cx.notify();
}
