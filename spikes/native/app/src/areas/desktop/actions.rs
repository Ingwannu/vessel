//! Menus and the sole shortcut engine share supported native window operations.

use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_view::desktop::zoom::ZoomChange;
use vessel_app_view::shortcuts::desktop::DesktopAction;

impl Shell {
    /// Native clicks and menus respect the sole shortcut recorder's admission.
    #[must_use]
    pub(crate) fn desktop_commands_admitted(&self) -> bool {
        !self.areas.shortcuts.page.recording()
    }

    /// Called by the recorder before capture/blur, not from rendering. Retire
    /// the old shared-menu lease before it can restore focus from a stale frame.
    pub(crate) fn desktop_recording_started(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.desktop.menu.close_and_restore(window, cx);
        self.areas.desktop.menu_cycle = std::rc::Rc::new(());
    }

    /// Capabilities use the fork's production window zoom, preserving native DPI.
    #[must_use]
    pub(crate) fn desktop_shortcut_supported(action: DesktopAction) -> bool {
        matches!(
            action,
            DesktopAction::NewWindow
                | DesktopAction::CloseWindow
                | DesktopAction::Fullscreen
                | DesktopAction::ZoomIn
                | DesktopAction::ZoomOut
                | DesktopAction::ActualSize
        )
    }

    /// Native menu clicks use exactly the same operation as matched shortcuts.
    pub(crate) fn run_desktop_shortcut(
        &mut self,
        action: DesktopAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if !self.desktop_commands_admitted() || !Self::desktop_shortcut_supported(action) {
            return false;
        }
        match action {
            DesktopAction::NewWindow => {
                return crate::host_windows::open_additional(cx)
                    == crate::host_windows::Delivery::Delivered;
            }
            DesktopAction::CloseWindow => vessel_app_kit::chrome::request_close(window, cx),
            DesktopAction::Fullscreen => window.toggle_fullscreen(),
            DesktopAction::ZoomIn => Self::queue_desktop_zoom(ZoomChange::In, window, cx),
            DesktopAction::ZoomOut => Self::queue_desktop_zoom(ZoomChange::Out, window, cx),
            DesktopAction::ActualSize => Self::queue_desktop_zoom(ZoomChange::Reset, window, cx),
        }
        true
    }

    fn queue_desktop_zoom(change: ZoomChange, window: &Window, cx: &mut Context<'_, Self>) {
        let handle = window.window_handle();
        let recipient = cx.entity().downgrade();
        // set_zoom synchronously runs bounds observers, which update this Shell.
        // Apply after its mutable borrow ends, then recheck both root identity
        // and recorder admission before changing the model or platform window.
        cx.defer(move |cx| {
            let _closed = handle.update(cx, |root, window, cx| {
                let Some(shell) = recipient.upgrade() else {
                    return;
                };
                if !vessel_app_kit::root_contains_view(&root, shell.entity_id(), cx) {
                    return;
                }
                let factor = shell.update(cx, |shell, _cx| {
                    if !shell.desktop_commands_admitted() {
                        return None;
                    }
                    shell.areas.desktop.zoom.adjust(change);
                    Some(shell.areas.desktop.zoom.factor())
                });
                if let Some(factor) = factor {
                    window.set_zoom(factor, cx);
                }
            });
        });
    }
}
