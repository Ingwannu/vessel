//! Typed desktop host commands shared by native menus and the one shortcut
//! interceptor. Window lifecycle, fullscreen and scaling stay with the host.

/// The desktop shell reports whether the requested action was performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DesktopAction {
    /// Open another native window through the host's existing lifecycle.
    NewWindow,
    /// Close only the current native window.
    CloseWindow,
    /// Toggle the native window's fullscreen state.
    Fullscreen,
    /// Increase app zoom through the host's bounded scale setting.
    ZoomIn,
    /// Decrease app zoom through the host's bounded scale setting.
    ZoomOut,
    /// Restore the host's default app scale.
    ActualSize,
}
