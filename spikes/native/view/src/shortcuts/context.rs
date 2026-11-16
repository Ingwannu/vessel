//! Pure focus and admission policy shared by the interceptor, help and browser
//! forwarding. Platform predicates belong to bindings; IME and overlay ownership
//! belong to the event boundary and cannot be weakened by a user override.

use super::Platform;

/// The focused surface, determined by the owning UI rather than guessed ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusScope {
    /// An actual terminal surface owns terminal editing keys.
    Terminal,
    /// The command center's search or result list.
    CommandCenter,
    /// The active agent composer.
    MessageInput,
    /// Another native text editor.
    Editable,
    /// An embedded browser surface.
    Browser,
    /// A non-editing app surface.
    Other,
}

/// Browser and desktop have different reserved accelerators and index keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    /// Native app with host-owned menus.
    Desktop,
    /// Browser app; browser accelerators stay with the browser.
    Web,
}

/// Event admission cannot be overridden by shortcut preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Resolve the press against current bindings.
    Available,
    /// An input method owns the press.
    Composing,
    /// `AltGr` or another character-preferred input owns the press.
    CharacterInput,
    /// Settings is capturing keys for a new binding.
    Capturing,
    /// The window is hidden or inactive.
    Inactive,
    /// A menu, dialog or popover owns keyboard focus.
    Overlay,
}

/// Current context is recomputed at every chord step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutContext {
    /// Command/Control convention.
    pub platform: Platform,
    /// Host accelerator policy.
    pub host: HostKind,
    /// Actual focused surface.
    pub focus: FocusScope,
    /// Whether the command center is open even when another control has focus.
    pub command_center: CommandCenterState,
    /// Top-level event admission.
    pub admission: Admission,
}

/// Command-center visibility is separate from the input's focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCenterState {
    /// Commands may open it.
    Closed,
    /// Only bindings allowing the open center may run.
    Open,
}

/// Exclusions remain named and independent rather than opaque flag bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Exclusion {
    /// No exclusion.
    #[default]
    Permit,
    /// Exclude the surface.
    Exclude,
}

/// A default binding's complete predicate, shared by matching and help.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct When {
    /// Mac-only or non-Mac-only, absent for both.
    pub platform: Option<Platform>,
    /// Desktop-only or web-only, absent for both.
    pub host: Option<HostKind>,
    /// Protect text editing in the default binding; user overrides may drop it.
    pub editable: Exclusion,
    /// Protect terminal input.
    pub terminal: Exclusion,
    /// Protect command-center navigation.
    pub command_center: Exclusion,
    /// Explicit scopes, empty for every scope.
    pub focus: &'static [FocusScope],
}

impl When {
    /// A binding without platform or focus restrictions.
    pub const ANY: Self = Self {
        platform: None,
        host: None,
        editable: Exclusion::Permit,
        terminal: Exclusion::Permit,
        command_center: Exclusion::Permit,
        focus: &[],
    };

    /// Restrict a default to Command-key platforms.
    #[must_use]
    pub const fn mac(self) -> Self {
        Self {
            platform: Some(Platform::Mac),
            ..self
        }
    }

    /// Restrict a default to Control-key platforms.
    #[must_use]
    pub const fn non_mac(self) -> Self {
        Self {
            platform: Some(Platform::Other),
            ..self
        }
    }

    /// Native host menus own this default.
    #[must_use]
    pub const fn desktop(self) -> Self {
        Self {
            host: Some(HostKind::Desktop),
            ..self
        }
    }

    /// Browser-safe default, separate from desktop accelerators.
    #[must_use]
    pub const fn web(self) -> Self {
        Self {
            host: Some(HostKind::Web),
            ..self
        }
    }

    /// Leave terminal input with the focused terminal.
    #[must_use]
    pub const fn outside_terminal(self) -> Self {
        Self {
            terminal: Exclusion::Exclude,
            ..self
        }
    }

    /// Leave command-center navigation with its modal.
    #[must_use]
    pub const fn outside_center(self) -> Self {
        Self {
            command_center: Exclusion::Exclude,
            ..self
        }
    }

    /// Protect native text editing on the shipped combo.
    #[must_use]
    pub const fn outside_editing(self) -> Self {
        Self {
            editable: Exclusion::Exclude,
            ..self
        }
    }

    /// Explicit surface restriction, retained even after rebinding.
    #[must_use]
    pub const fn scopes(self, focus: &'static [FocusScope]) -> Self {
        Self { focus, ..self }
    }

    /// Platform-only filtering is also used by the help and rebind rows.
    #[must_use]
    pub fn on_platform(self, context: ShortcutContext) -> bool {
        self.platform
            .is_none_or(|platform| platform == context.platform)
            && self.host.is_none_or(|host| host == context.host)
    }

    /// Recheck all guards at each step: a pending chord grants no authority
    /// after focus moves to an excluded editor, terminal or overlay.
    #[must_use]
    pub fn matches(self, context: ShortcutContext) -> bool {
        context.admission == Admission::Available
            && self.on_platform(context)
            && !(self.editable == Exclusion::Exclude
                && matches!(
                    context.focus,
                    FocusScope::MessageInput | FocusScope::Editable
                ))
            && !(self.terminal == Exclusion::Exclude && context.focus == FocusScope::Terminal)
            && !(self.command_center == Exclusion::Exclude
                && context.command_center == CommandCenterState::Open)
            && (self.focus.is_empty() || self.focus.contains(&context.focus))
    }

    /// Paseo removes only the default editing guard when a combo is overridden.
    /// IME, terminal, platform, overlay and explicit focus rules remain intact.
    #[must_use]
    pub const fn overridden(self) -> Self {
        Self {
            editable: Exclusion::Permit,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT: ShortcutContext = ShortcutContext {
        platform: Platform::Other,
        host: HostKind::Desktop,
        focus: FocusScope::Other,
        command_center: CommandCenterState::Closed,
        admission: Admission::Available,
    };

    #[test]
    fn every_admission_exclusion_blocks_even_a_default_with_no_predicates() {
        for admission in [
            Admission::Composing,
            Admission::CharacterInput,
            Admission::Capturing,
            Admission::Inactive,
            Admission::Overlay,
        ] {
            assert!(!When::default().matches(ShortcutContext {
                admission,
                ..CONTEXT
            }));
        }
    }

    #[test]
    fn platform_host_and_scope_guards_are_independent() {
        let when = When {
            platform: Some(Platform::Mac),
            host: Some(HostKind::Web),
            focus: &[FocusScope::Browser],
            ..When::default()
        };
        assert!(!when.matches(CONTEXT));
        let context = ShortcutContext {
            platform: Platform::Mac,
            host: HostKind::Web,
            focus: FocusScope::Browser,
            ..CONTEXT
        };
        assert!(when.matches(context));
        assert!(!when.matches(ShortcutContext {
            focus: FocusScope::Other,
            ..context
        }));
    }

    #[test]
    fn overrides_remove_only_default_editing_exclusion() {
        let when = When {
            editable: Exclusion::Exclude,
            terminal: Exclusion::Exclude,
            command_center: Exclusion::Exclude,
            ..When::default()
        };
        for focus in [FocusScope::MessageInput, FocusScope::Editable] {
            let context = ShortcutContext { focus, ..CONTEXT };
            assert!(!when.matches(context));
            assert!(when.overridden().matches(context));
        }
        assert!(!when.overridden().matches(ShortcutContext {
            focus: FocusScope::Terminal,
            ..CONTEXT
        }));
        assert!(!when.overridden().matches(ShortcutContext {
            command_center: CommandCenterState::Open,
            ..CONTEXT
        }));
    }
}
