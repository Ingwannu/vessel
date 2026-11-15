//! The first window restores state; additional windows have independent placement.

use super::{Desktop, WindowClose, WindowRecord};
use crate::host_windows::Registry;
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
    let _delivery = open(Placement::Primary, launch, cx);
}
pub(crate) fn open_additional(cx: &mut App) -> crate::host_windows::Delivery {
    open(Placement::Additional, None, cx)
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
    let registry = cx.default_global::<Registry>();
    let Some(record) = registry
        .records
        .iter_mut()
        .find(|record| record.window == window.window_handle())
    else {
        return true;
    };
    if record.close.is_checking() {
        return false;
    }
    record.close = WindowClose::begin();
    let record = record.clone();
    // Shortcut/menu close can run while its Shell is mutably borrowed. Reading
    // its documents here would reborrow and panic, so admission runs next turn.
    cx.defer(move |cx| {
        let current = record
            .window
            .update(cx, |root, _window, cx| current_record(&record, &root, cx))
            .unwrap_or(false);
        if !current {
            reset_close(&record, cx);
            return;
        }
        if !cfg!(target_os = "macos") && cx.windows().len() <= 1 {
            reset_close(&record, cx);
            super::request_quit(cx);
            return;
        }
        let result = record.window.update(cx, |root, window, cx| {
            if !current_record(&record, &root, cx) {
                return;
            }
            let admitted = record
                .shell
                .upgrade()
                .is_some_and(|shell| shell.update(cx, handler));
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

fn current_record(record: &WindowRecord, root: &gpui::AnyView, cx: &App) -> bool {
    cx.try_global::<Registry>().is_some_and(|registry| {
        registry.records.iter().any(|current| {
            current.window == record.window
                && current.shell.entity_id() == record.shell.entity_id()
                && current.close.same_check(&record.close)
        })
    }) && record.shell.upgrade().is_some()
        && vessel_app_kit::root_contains_view(root, record.shell.entity_id(), cx)
}

fn reset_close(record: &WindowRecord, cx: &mut App) {
    if let Some(retained) = cx
        .default_global::<Registry>()
        .records
        .iter_mut()
        .find(|retained| {
            retained.window == record.window
                && retained.shell.entity_id() == record.shell.entity_id()
                && retained.close.same_check(&record.close)
        })
    {
        retained.close = WindowClose::Idle;
    }
}

fn open(
    placement: Placement,
    launch: Option<Launch>,
    cx: &mut App,
) -> crate::host_windows::Delivery {
    let prefs = (*cx.global::<Desktop>().preferences.current()).clone();
    let language = crate::language::Language::new();
    let words = match language.prepare(prefs.settings.language) {
        Ok(words) => words,
        Err(error) => {
            tracing::error!(%error,"window translations could not start");
            return crate::host_windows::Delivery::Unavailable;
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
        // No Shell observer exists yet, so restoration cannot reborrow it.
        window.set_zoom(state.zoom().factor(), cx);
        crate::motion::note_renderer(window, cx);
        let shell = cx.new(|cx| Shell::new(prefs, language, words, window, cx));
        crate::host_windows::register(&shell, window, cx);
        shell.update(cx, |shell, cx| {
            shell.areas.desktop.zoom = state.zoom();
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
        Ok(_opened) => {
            cx.activate(true);
            crate::host_windows::Delivery::Delivered
        }
        Err(error) => {
            tracing::error!(%error,"native window could not open");
            crate::host_windows::Delivery::Unavailable
        }
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
        Launch::NewWindow => {
            let _delivery = crate::host_windows::open_additional(cx);
        }
        Launch::Project(directory) => {
            let _delivery = open(Placement::Additional, Some(Launch::Project(directory)), cx);
        }
        session @ Launch::Session(_) => {
            let pending = session.clone();
            let delivery = crate::host_windows::with_focused_shell(cx, move |shell, window, cx| {
                window.activate_window();
                shell.areas.desktop.pending.push(pending);
                cx.notify();
            });
            if delivery == crate::host_windows::Delivery::Unavailable {
                open_primary(Some(session), cx);
            }
        }
    }
}

pub(crate) fn watch_window(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    remember(shell, window, cx);
    cx.observe_window_bounds(window, |shell, window, cx| remember(shell, window, cx))
        .detach();
}

fn remember(shell: &Shell, window: &Window, cx: &mut App) {
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
    cx.global_mut::<Desktop>()
        .remember(state.with_zoom(shell.areas.desktop.zoom));
}
