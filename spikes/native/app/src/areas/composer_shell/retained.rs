//! Staged owner adapter over W12's sole resource/admission registry.
//! Opaque tickets are supplied by that registry; this allocates no identity and
//! grants no network authority. W6 keeps message ids, payloads and optimistic rows.
use vessel_app_kit::{
    TextField,
    ui::{App, Window},
};
use vessel_app_view::{
    composer_shell::surface::{Extent, Overlay, Surface},
    residents::{
        FollowBinding, FollowRetirement, HoldReason, MAX_RESIDENT_OPERATIONS, OperationKind,
        OperationTicket,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectState {
    Dispatched,
    Uncertain,
}
#[derive(Debug, Clone, Copy)]
struct Effect {
    binding: FollowBinding,
    ticket: OperationTicket,
    state: EffectState,
}

/// Outcome projection after the sole registry admitted the exact settlement.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Settled {
    Accepted,
    Refused,
    Unknown,
}

/// Only a clean empty editor may be dropped after transferring this snapshot.
/// Dirty text, IME, focus, overlays and unresolved effects retain the live field.
/// Undo history of a clean, inactive empty field is not serialized.
pub(crate) struct CleanSnapshot {
    surface: Surface,
}
impl CleanSnapshot {
    pub(crate) const fn surface(&self) -> Surface {
        self.surface
    }
}

#[derive(Default)]
pub(crate) struct ResidentComposer {
    pub(crate) surface: Surface,
    effects: Vec<Effect>,
}
impl ResidentComposer {
    /// Record an already admitted dispatch, without creating another send path.
    pub(crate) fn track(&mut self, binding: FollowBinding, ticket: OperationTicket) -> bool {
        if self.effects.len() >= MAX_RESIDENT_OPERATIONS
            || self.effects.iter().any(|effect| effect.ticket == ticket)
        {
            return false;
        }
        self.effects.push(Effect {
            binding,
            ticket,
            state: EffectState::Dispatched,
        });
        true
    }
    /// Call only inside `SessionResidents::settle`, after ticket consumption.
    /// Unknown and retired effects preserve all draft/payload state.
    pub(crate) fn settled(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
        outcome: Settled,
    ) -> bool {
        let Some(index) = self.effects.iter().position(|effect| {
            effect.binding == binding
                && effect.ticket == ticket
                && effect.state == EffectState::Dispatched
        }) else {
            return false;
        };
        match outcome {
            Settled::Accepted | Settled::Refused => {
                let _effect = self.effects.remove(index);
            }
            Settled::Unknown => self.effects[index].state = EffectState::Uncertain,
        }
        true
    }
    /// Retirement already invalidated the binding. Mark only the matching owner
    /// tickets uncertain; never clear text, retry, or fabricate a new admission.
    pub(crate) fn retired(&mut self, notice: &FollowRetirement) {
        for operation in notice.operations() {
            if operation.kind() != OperationKind::Composer {
                continue;
            }
            for effect in &mut self.effects {
                if effect.binding == notice.binding() && effect.ticket == operation.ticket() {
                    effect.state = EffectState::Uncertain;
                }
            }
        }
    }
    fn effect_hold(&self) -> Option<HoldReason> {
        if self
            .effects
            .iter()
            .any(|effect| effect.state == EffectState::Uncertain)
        {
            Some(HoldReason::UncertainEffect)
        } else if self.effects.is_empty() {
            None
        } else {
            Some(HoldReason::PendingEffect)
        }
    }
    pub(crate) fn snapshot(
        &self,
        field: &TextField,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<CleanSnapshot, HoldReason> {
        if let Some(reason) = self.effect_hold() {
            return Err(reason);
        }
        if field.composing(window, cx) {
            return Err(HoldReason::Ime);
        }
        if field.focus_handle(cx).is_focused(window) {
            return Err(HoldReason::ActiveEditor);
        }
        if self.surface.extent() != Extent::Inline || self.surface.overlay() != Overlay::None {
            return Err(HoldReason::Overlay);
        }
        if !field.text(cx).is_empty() || field.selection(cx) != (0..0) {
            return Err(HoldReason::DirtyInput);
        }
        Ok(CleanSnapshot {
            surface: self.surface,
        })
    }
}

#[cfg(test)]
mod tests;
