//! The process has one list of live windows and their weak Shell recipients.
//!
//! Window discovery must be shared by desktop menus, lifecycle admission and
//! notification routing. Keeping the record here lets those consumers reuse
//! the same owner without depending on desktop persistence or node startup.

use crate::shell::Shell;
use gpui::{AnyWindowHandle, Global, WeakEntity};

#[derive(Clone)]
pub(crate) struct WindowRecord {
    pub(crate) window: AnyWindowHandle,
    pub(crate) shell: WeakEntity<Shell>,
    pub(crate) close: WindowClose,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowClose {
    Idle,
    Checking,
}

#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) records: Vec<WindowRecord>,
}

impl Global for Registry {}
