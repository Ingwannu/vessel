//! A browser has no authority to launch or inspect a desktop-local process.

pub(crate) fn dialog(
    _shell: &crate::shell::Shell,
    _tokens: &vessel_app_kit::Tokens,
    _window: &mut gpui::Window,
    _cx: &mut gpui::Context<'_, crate::shell::Shell>,
) -> Option<gpui::AnyElement> {
    None
}
