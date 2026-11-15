//! Capture and confirmation are event-time operations. Passive registry or
//! preference changes are revalidated before committing the reviewed key move.

use super::{Asking, RowAct};
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::prefs::{ActionId, PreferenceChange, ShortcutEdit, ShortcutEdits};
use vessel_app_view::shortcuts::context::{Admission, ShortcutContext};
use vessel_app_view::shortcuts::engine::Payload;
use vessel_app_view::shortcuts::sequence_capture::{CaptureRole, CaptureVerdict, SequenceCapture};
use vessel_app_view::shortcuts::{
    Chord, Platform, action_label, chord_caps, default_context, definitions, effective_bindings,
    rebinding,
};

fn context() -> ShortcutContext {
    default_context(Platform::native())
}

pub(super) fn verdict(shell: &Shell, capture: &SequenceCapture) -> CaptureVerdict {
    let Ok(definitions) = definitions() else {
        return CaptureVerdict::Waiting;
    };
    let Some(definition) = definitions
        .iter()
        .find(|definition| definition.id == capture.binding)
    else {
        return CaptureVerdict::Waiting;
    };
    let Ok(bindings) = effective_bindings(&shell.prefs.settings.shortcuts, context()) else {
        return CaptureVerdict::Waiting;
    };
    capture.verdict(definition.when, context(), &bindings)
}

impl Shell {
    pub(in crate::areas::shortcuts) fn shortcut_recorded(
        &mut self,
        chord: Chord,
        cx: &mut Context<'_, Self>,
    ) {
        let bare = !(chord.primary || chord.ctrl || chord.alt || chord.shift || chord.meta);
        if let Some(capture) = &mut self.areas.shortcuts.page.capture {
            if bare && chord.key == "Backspace" {
                capture.back();
            } else {
                capture.press(
                    chord,
                    ShortcutContext {
                        admission: Admission::Capturing,
                        ..context()
                    },
                );
            }
        }
        cx.notify();
    }

    pub(super) fn shortcut_menu(
        &mut self,
        id: ActionId,
        at: vessel_app_kit::ui::Point<vessel_app_kit::ui::Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let page = &mut self.areas.shortcuts.page;
        page.capture = None;
        page.menu_for = Some(id);
        page.menu.open(at, window, cx);
        cx.notify();
    }

    pub(super) fn shortcut_act(
        &mut self,
        act: RowAct,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(id) = self.areas.shortcuts.page.menu_for.take() else {
            return;
        };
        match act {
            RowAct::Record => {
                let Ok(definitions) = definitions() else {
                    return;
                };
                let Some(definition) = definitions.iter().find(|definition| definition.id == id)
                else {
                    return;
                };
                let role = if definition.payload == Payload::Index {
                    CaptureRole::Index
                } else {
                    CaptureRole::Ordinary
                };
                super::super::badges::clear(&mut self.areas.shortcuts);
                self.areas.shortcuts.page.capture = Some(SequenceCapture::new(id, role));
                window.blur(cx);
            }
            RowAct::Clear | RowAct::Reset => match rebinding::edit(
                &self.prefs.settings.shortcuts,
                context(),
                &id,
                None,
                act == RowAct::Reset,
            ) {
                Ok(edits) => {
                    let _saved = self.shortcut_edits(edits, window, cx);
                }
                Err(error) => tracing::warn!(%error, "shortcut edit refused"),
            },
        }
        cx.notify();
    }

    pub(super) fn shortcut_done(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(capture) = self.areas.shortcuts.page.capture.clone() else {
            return;
        };
        self.shortcut_review(capture, None, window, cx);
    }

    fn shortcut_review(
        &mut self,
        capture: SequenceCapture,
        reviewed: Option<&[ActionId]>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let CaptureVerdict::Ready { keys, conflicts } = verdict(self, &capture) else {
            return;
        };
        if !conflicts.is_empty() && reviewed != Some(conflicts.as_slice()) {
            let sentences = conflict_sentences(self, &keys, &conflicts);
            self.areas.shortcuts.page.capture = None;
            self.areas.shortcuts.page.asking = Some(Asking {
                capture,
                conflicts,
                sentences,
            });
            self.areas.shortcuts.page.dialog.focus(window, cx);
        } else {
            let Ok(bindings) = effective_bindings(&self.prefs.settings.shortcuts, context()) else {
                return;
            };
            match rebinding::reassign(
                &self.prefs.settings.shortcuts,
                context(),
                &capture.binding,
                &keys,
                &conflicts,
                &bindings,
            ) {
                Ok(edits) => {
                    if self.shortcut_edits(edits, window, cx) {
                        self.areas.shortcuts.page.capture = None;
                    } else {
                        self.areas.shortcuts.page.capture = Some(capture);
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "shortcut reassignment refused");
                    self.areas.shortcuts.page.capture = Some(capture);
                    self.shortcut_save_failed(cx);
                }
            }
        }
        cx.notify();
    }

    pub(super) fn shortcut_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.areas.shortcuts.page.capture = None;
        cx.notify();
    }

    pub(super) fn shortcut_reassign(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(asking) = self.areas.shortcuts.page.asking.take() {
            self.shortcut_review(asking.capture, Some(&asking.conflicts), window, cx);
        }
    }

    pub(super) fn shortcut_unask(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(asking) = self.areas.shortcuts.page.asking.take() {
            self.areas.shortcuts.page.capture = Some(asking.capture);
        }
        cx.notify();
    }

    fn shortcut_edits(
        &mut self,
        edits: Vec<ShortcutEdit>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if edits.is_empty() {
            return true;
        }
        let Ok(edits) = ShortcutEdits::try_from(edits) else {
            self.shortcut_save_failed(cx);
            return false;
        };
        let change = PreferenceChange::Shortcuts { edits };
        let Ok(expected) = self.prefs.settings.changed(&change) else {
            self.shortcut_save_failed(cx);
            return false;
        };
        // This is local acceptance only. The shared desktop preference
        // coordinator owns write acknowledgement, rollback and cross-window merge.
        self.prefer(&change, window, cx);
        let applied = self.prefs.settings.shortcuts == expected.shortcuts;
        if !applied {
            self.shortcut_save_failed(cx);
        }
        applied
    }

    fn shortcut_save_failed(&mut self, cx: &mut Context<'_, Self>) {
        self.toast(
            self.words.shortcuts().save_failed().to_owned(),
            vessel_app_view::toasts::ToastTone::Error,
            cx,
        );
    }

    pub(in crate::areas::shortcuts) fn refresh_shortcut_conflicts(&mut self) {
        let Some(asking) = self.areas.shortcuts.page.asking.as_ref() else {
            return;
        };
        let CaptureVerdict::Ready { keys, .. } = verdict(self, &asking.capture) else {
            return;
        };
        let sentences = conflict_sentences(self, &keys, &asking.conflicts);
        if let Some(asking) = self.areas.shortcuts.page.asking.as_mut() {
            asking.sentences = sentences;
        }
    }
}

fn conflict_sentences(
    shell: &Shell,
    keys: &vessel_app_view::shortcuts::sequence::KeySequence,
    conflicts: &[ActionId],
) -> Vec<String> {
    let platform = Platform::native();
    let key = keys
        .chords()
        .map(|chord| chord_caps(chord, platform).join("+"))
        .collect::<Vec<_>>()
        .join(&format!(" {} ", shell.words.shortcuts().then()));
    let Ok(definitions) = definitions() else {
        return Vec::new();
    };
    conflicts
        .iter()
        .filter_map(|id| {
            let definition = definitions.iter().find(|definition| &definition.id == id)?;
            let action = action_label(definition.action, shell.words);
            shell.words.shortcuts().conflict(action, &key).ok()
        })
        .collect()
}
