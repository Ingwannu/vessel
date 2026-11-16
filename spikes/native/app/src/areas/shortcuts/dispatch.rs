//! Native callbacks for the pure priority dispatcher. Feature owners supply
//! their typed adapters; an absent/inactive handler leaves the key untouched.
//! Candidate callbacks are snapshotted before touching Shell so handlers may
//! safely replace registrations while navigating or closing a pane.

use std::collections::BTreeMap;

use vessel_app_kit::ui::{App, Context, Window};
use vessel_app_view::shortcuts::dispatcher::{
    ActionScope, DispatchError, Dispatcher, Handled, Handler, HandlerId, HandlerOwner, Priority,
};
use vessel_app_view::shortcuts::{
    Action,
    context::ShortcutContext,
    engine::{KeyRepeat, ShortcutIndex},
};

use crate::shell::Shell;

/// The same request reaches keyboard and command-center feature handlers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShortcutRequest {
    pub(crate) repeat: KeyRepeat,
    pub(crate) action: Action,
    pub(crate) scope: ActionScope,
    pub(crate) index: Option<ShortcutIndex>,
    pub(crate) context: ShortcutContext,
}

/// Feature-owned callbacks never resolve an action id from a string.
#[derive(Clone, Copy)]
pub(crate) struct Callbacks {
    pub(crate) enabled: fn(&Shell, &ShortcutRequest, &Window, &App) -> bool,
    pub(crate) perform:
        fn(&mut Shell, ShortcutRequest, &mut Window, &mut Context<'_, Shell>) -> Handled,
}

/// One feature instance owns its identity and the actions it can actually perform.
pub(crate) struct KeyboardHandler {
    pub(crate) id: HandlerId,
    pub(crate) actions: Vec<Action>,
    pub(crate) priority: Priority,
    pub(crate) callbacks: Callbacks,
}

/// Stale cleanup cannot remove a replacement's callback or pure registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Registration {
    id: HandlerId,
    owner: HandlerOwner,
}

struct Entry {
    owner: HandlerOwner,
    callbacks: Callbacks,
}
struct Candidate {
    registration: Registration,
    callbacks: Callbacks,
}

/// One native adapter around the already tested pure ordering policy.
#[derive(Default)]
pub(super) struct NativeDispatcher {
    core: Dispatcher<Action>,
    callbacks: BTreeMap<HandlerId, Entry>,
}

impl NativeDispatcher {
    fn register(&mut self, handler: KeyboardHandler) -> Result<Registration, DispatchError> {
        let id = handler.id;
        let owner = self.core.register(Handler {
            id: id.clone(),
            actions: handler.actions,
            priority: handler.priority,
        })?;
        drop(self.callbacks.insert(
            id.clone(),
            Entry {
                owner: owner.clone(),
                callbacks: handler.callbacks,
            },
        ));
        Ok(Registration { id, owner })
    }

    fn remove(&mut self, registration: &Registration) {
        self.core.remove(&registration.owner);
        if self.current(registration) {
            drop(self.callbacks.remove(&registration.id));
        }
    }

    fn current(&self, registration: &Registration) -> bool {
        self.callbacks
            .get(&registration.id)
            .is_some_and(|entry| entry.owner == registration.owner)
    }

    fn candidates(&self, action: Action) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        let _unhandled = self.core.dispatch(
            &action,
            |_| true,
            |id, _| {
                if let Some(entry) = self.callbacks.get(id) {
                    candidates.push(Candidate {
                        registration: Registration {
                            id: id.clone(),
                            owner: entry.owner.clone(),
                        },
                        callbacks: entry.callbacks,
                    });
                }
                Handled::No
            },
        );
        candidates
    }
}

impl Shell {
    /// A command-center action is already typed, so it skips chord matching but
    /// retains current admission and the same ordered owner callbacks. Compact
    /// pointer commands remain usable even where global keyboard input is off.
    pub(crate) fn dispatch_shortcut_command(
        &mut self,
        action: Action,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if !window.is_visible()
            || self.areas.shortcuts.page.recording()
            || vessel_app_kit::TextField::focused_composing(window, cx)
        {
            return false;
        }
        let context = self.shortcut_context(&window.context_stack(), window, cx);
        if context.admission != vessel_app_view::shortcuts::context::Admission::Available {
            return false;
        }
        self.dispatch_shortcut_to_owners(
            ShortcutRequest {
                action,
                scope: action.scope(),
                index: None,
                repeat: KeyRepeat::Initial,
                context,
            },
            window,
            cx,
        )
    }

    /// A retained feature registers once and removes its own token on teardown.
    pub(crate) fn register_shortcut_handler(
        &mut self,
        handler: KeyboardHandler,
    ) -> Result<Registration, DispatchError> {
        self.areas.shortcuts.dispatch.register(handler)
    }

    pub(super) fn shortcut_handler_available(
        &self,
        request: &ShortcutRequest,
        window: &Window,
        cx: &App,
    ) -> bool {
        self.areas
            .shortcuts
            .dispatch
            .candidates(request.action)
            .into_iter()
            .any(|candidate| {
                self.areas
                    .shortcuts
                    .dispatch
                    .current(&candidate.registration)
                    && (candidate.callbacks.enabled)(self, request, window, cx)
            })
    }

    /// Replacement registrations survive an older feature's cleanup.
    pub(crate) fn remove_shortcut_handler(&mut self, registration: &Registration) {
        self.areas.shortcuts.dispatch.remove(registration);
    }

    pub(crate) fn dispatch_shortcut_to_owners(
        &mut self,
        request: ShortcutRequest,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let candidates = self.areas.shortcuts.dispatch.candidates(request.action);
        for candidate in candidates {
            if self
                .areas
                .shortcuts
                .dispatch
                .current(&candidate.registration)
                && (candidate.callbacks.enabled)(self, &request, window, cx)
                && (candidate.callbacks.perform)(self, request, window, cx) == Handled::Yes
            {
                if request.action.exits_focus_mode() {
                    self.layout_exit_focus(cx);
                }
                return true;
            }
        }
        false
    }
}

pub(super) fn register_core(shell: &mut Shell) -> Result<Registration, DispatchError> {
    shell.register_shortcut_handler(KeyboardHandler {
        id: "native-core".parse()?,
        actions: Action::ALL
            .into_iter()
            .filter(|action| action.has_builtin_fallback())
            .collect(),
        priority: Priority::default(),
        callbacks: Callbacks {
            enabled: core_enabled,
            perform: core_perform,
        },
    })
}

fn core_enabled(shell: &Shell, request: &ShortcutRequest, _window: &Window, _cx: &App) -> bool {
    if request.context.admission != vessel_app_view::shortcuts::context::Admission::Available {
        return false;
    }
    match request.scope {
        ActionScope::Global | ActionScope::Sidebar => true,
        ActionScope::MessageInput => matches!(
            shell.route,
            crate::routes::Route::Session | crate::routes::Route::Draft
        ),
        ActionScope::Workspace => shell.route == crate::routes::Route::Session,
    }
}

fn core_perform(
    shell: &mut Shell,
    request: ShortcutRequest,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Handled {
    if shell.run_shortcut(
        request.action,
        request.index.map(ShortcutIndex::get),
        request.repeat == KeyRepeat::Repeated,
        window,
        cx,
    ) {
        Handled::Yes
    } else {
        Handled::No
    }
}
