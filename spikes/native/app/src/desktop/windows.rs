//! The first window restores state; additional windows have independent placement.

use super::{Desktop, WindowClose, WindowRecord};
use crate::shell::Shell;
use gpui::{App, AppContext as _, Bounds, Context, Pixels, Window, WindowBounds, point, size};
use vessel_app_link::desktop::instance::Launch;
use vessel_app_view::desktop::window::{
    MIN_HEIGHT, MIN_WIDTH, Presentation, WindowState, WorkArea,
};

#[cfg(all(test, not(target_os = "macos")))]
mod tests;

#[derive(Clone, Copy)]
enum Placement {
    Primary,
    Additional,
}

pub(super) fn open_primary(launch: Option<Launch>, cx: &mut App) {
    open(Placement::Primary, launch, cx);
}
pub(crate) fn open_additional(cx: &mut App) {
    open(Placement::Additional, None, cx);
}

/// Last-window removal waits for the same owned-node quit boundary as the menu.
pub(super) fn request_close(window: &mut Window, cx: &mut App) {
    if may_close(window, cx) {
        window.remove_window();
    }
}

fn may_close(window: &mut Window, cx: &mut App) -> bool {
    if !cfg!(target_os = "macos") && cx.windows().len() <= 1 {
        super::request_quit(cx);
        false
    } else {
        guarded_close(window, cx)
    }
}

fn guarded_close(window: &Window, cx: &mut App) -> bool {
    let Some(handler) = cx
        .try_global::<Desktop>()
        .and_then(|desktop| desktop.window_close)
    else {
        return true;
    };
    let desktop = cx.global_mut::<Desktop>();
    let Some(record) = desktop
        .records
        .iter_mut()
        .find(|record| record.window == window.window_handle())
    else {
        return true;
    };
    if record.close == WindowClose::Checking {
        return false;
    }
    record.close = WindowClose::Checking;
    let record = record.clone();
    // Shortcut/menu close can run while its Shell is mutably borrowed. Reading
    // its documents here would reborrow and panic, so admission runs next turn.
    cx.defer(move |cx| {
        if !cfg!(target_os = "macos") && cx.windows().len() <= 1 {
            reset_close(&record, cx);
            super::request_quit(cx);
            return;
        }
        let result = record.window.update(cx, |_root, window, cx| {
            let admitted = record
                .shell
                .upgrade()
                .is_none_or(|shell| shell.update(cx, handler));
            if admitted {
                if !cfg!(target_os = "macos") && cx.windows().len() <= 1 {
                    super::request_quit(cx);
                } else {
                    window.remove_window();
                }
            }
        });
        reset_close(&record, cx);
        if let Err(error) = result {
            tracing::warn!(%error,"window close target is no longer available");
        }
    });
    false
}

fn reset_close(record: &WindowRecord, cx: &mut App) {
    if let Some(retained) = cx
        .global_mut::<Desktop>()
        .records
        .iter_mut()
        .find(|retained| retained.window == record.window)
    {
        retained.close = WindowClose::Idle;
    }
}

fn open(placement: Placement, launch: Option<Launch>, cx: &mut App) {
    let prefs = (*cx.global::<Desktop>().preferences.current()).clone();
    let language = crate::language::Language::new();
    let words = match language.prepare(prefs.settings.language) {
        Ok(words) => words,
        Err(error) => {
            tracing::error!(%error,"window translations could not start");
            return;
        }
    };
    let state = match placement {
        Placement::Primary => cx.global::<Desktop>().saved.unwrap_or_default(),
        Placement::Additional => WindowState::default(),
    };
    let areas: Vec<_> = cx
        .displays()
        .iter()
        .filter_map(|display| {
            let bounds = display.visible_bounds();
            WorkArea::new(
                bounds.origin.x.into(),
                bounds.origin.y.into(),
                bounds.size.width.into(),
                bounds.size.height.into(),
            )
        })
        .collect();
    let state = state.clamp(&areas);
    let (width, height) = state.dimensions();
    let bounds = match state.position() {
        Some((x, y)) => Bounds::new(
            point(Pixels::from(x), Pixels::from(y)),
            size(Pixels::from(width), Pixels::from(height)),
        ),
        None => Bounds::centered(None, size(Pixels::from(width), Pixels::from(height)), cx),
    };
    let mut options =
        vessel_app_kit::chrome::window_options(format!("Vessel {}", crate::LABEL), bounds);
    options.window_min_size = Some(size(Pixels::from(MIN_WIDTH), Pixels::from(MIN_HEIGHT)));
    options.window_bounds = Some(match state.presentation() {
        Presentation::Windowed => WindowBounds::Windowed(bounds),
        Presentation::Maximized => WindowBounds::Maximized(bounds),
        Presentation::Fullscreen => WindowBounds::Fullscreen(bounds),
    });
    if let Some((width, height)) = capture_size() {
        options.window_min_size = None;
        options.window_bounds = Some(WindowBounds::centered(
            size(Pixels::from(width), Pixels::from(height)),
            cx,
        ));
    }
    let primary = matches!(placement, Placement::Primary);
    match cx.open_window(options, move |window, cx| {
        window.on_window_should_close(cx, may_close);
        crate::motion::note_renderer(window, cx);
        let shell = cx.new(|cx| Shell::new(prefs, language, words, window, cx));
        cx.global_mut::<Desktop>().records.push(WindowRecord {
            window: window.window_handle(),
            shell: shell.downgrade(),
            close: WindowClose::Idle,
        });
        shell.update(cx, |shell, cx| {
            cx.observe_window_activation(window, |shell, window, _cx| {
                if window.is_window_active() {
                    crate::update::check_on_focus(shell);
                }
            })
            .detach();
            if primary {
                watch_window(shell, window, cx);
            }
            if let Some(launch) = launch {
                shell.areas.desktop.pending.push(launch);
            }
        });
        vessel_app_kit::root(shell, window, cx)
    }) {
        Ok(_opened) => cx.activate(true),
        Err(error) => tracing::error!(%error,"native window could not open"),
    }
}

fn capture_size() -> Option<(f64, f64)> {
    let text = std::env::var("VESSEL_APP_SIZE").ok()?;
    let (width, height) = text.split_once('x')?;
    let width = width.parse::<f64>().ok()?;
    let height = height.parse::<f64>().ok()?;
    (width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0)
        .then_some((width, height))
}

pub(super) fn deliver(launch: Launch, cx: &mut App) {
    match launch {
        Launch::NewWindow => open_additional(cx),
        Launch::Project(directory) => {
            open(Placement::Additional, Some(Launch::Project(directory)), cx);
        }
        session @ Launch::Session(_) => {
            let target = super::target_window(cx);
            if let Some(record) = target {
                let result = record.window.update(cx, |_root, window, cx| {
                    window.activate_window();
                    if let Some(shell) = record.shell.upgrade() {
                        shell.update(cx, |shell, cx| {
                            shell.areas.desktop.pending.push(session);
                            cx.notify();
                        });
                    }
                });
                if let Err(error) = result {
                    tracing::warn!(%error,"desktop destination window is no longer available");
                }
            } else {
                open_primary(Some(session), cx);
            }
        }
    }
}

pub(crate) fn watch_window(_shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    remember(window, cx);
    cx.observe_window_bounds(window, |_shell, window, cx| remember(window, cx))
        .detach();
}

fn remember(window: &Window, cx: &mut App) {
    let native = window.window_bounds();
    let bounds = native.get_bounds();
    let presentation = match native {
        WindowBounds::Windowed(_) => Presentation::Windowed,
        WindowBounds::Maximized(_) => Presentation::Maximized,
        WindowBounds::Fullscreen(_) => Presentation::Fullscreen,
    };
    let Some(state) = WindowState::new(
        Some((bounds.origin.x.into(), bounds.origin.y.into())),
        bounds.size.width.into(),
        bounds.size.height.into(),
        presentation,
    ) else {
        return;
    };
    let desktop = cx.global_mut::<Desktop>();
    desktop.saved = Some(state);
    desktop.latest = state;
    if let Some(writer) = &desktop.writer
        && let Err(error) = writer.changed(state)
    {
        tracing::warn!(%error,"window position could not be queued for saving");
    }
}
