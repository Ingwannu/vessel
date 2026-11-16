//! One row carrier: explicit legacy surfaces or the exact admitted resource.
//! Derived origins retain their actual parent; scoped resolution never falls back.
use super::ConversationArea;
#[cfg(test)]
use super::ResidentSeat;
use super::area::Shown;
use super::render_scope::RenderScope;
use crate::shell::Shell;
use std::sync::Arc;
#[cfg(test)]
use vessel_app_view::residents::SessionContext;

/// Captured by rows and reusable callbacks; scoped state is never selected globally.
#[derive(Debug, Clone)]
pub(crate) enum Seat {
    Legacy {
        read: fn(&Shell) -> Option<&ConversationArea>,
        write: fn(&mut Shell) -> Option<&mut ConversationArea>,
    },
    Origin(Arc<OriginSeat>),
    #[cfg(test)]
    Admitted(Arc<AdmittedSeat>),
}

#[derive(Debug, Clone)]
pub(crate) struct OriginSeat {
    parent: Seat,
    scope: Option<RenderScope>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct AdmittedSeat {
    owner: ResidentSeat,
    scope: RenderScope,
}

impl Seat {
    /// Unconverted product session; removed from the main renderer at cutover.
    pub(crate) const SESSION: Self = Self::legacy(session, session_mut);
    /// Historical global origin, only for unconverted product surfaces.
    pub(crate) const ORIGIN: Self = Self::legacy(origin, origin_mut);

    /// Preserve independent unconverted surfaces without changing their owner.
    #[must_use]
    pub(crate) const fn legacy(
        read: fn(&Shell) -> Option<&ConversationArea>,
        write: fn(&mut Shell) -> Option<&mut ConversationArea>,
    ) -> Self {
        Self::Legacy { read, write }
    }

    /// Staged row adoption reuses the checked registry predicates, not a new map.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn admitted_main(context: SessionContext) -> Self {
        Self::Admitted(Arc::new(AdmittedSeat {
            scope: RenderScope::admitted(&context),
            owner: ResidentSeat::main(context),
        }))
    }

    /// One level from THIS parent. Repetition derives deeper origins recursively.
    #[must_use]
    pub(crate) fn origin(&self) -> Self {
        Self::Origin(Arc::new(OriginSeat {
            parent: self.clone(),
            scope: self.render_scope().map(|scope| scope.origin()),
        }))
    }

    pub(super) fn origin_parent(&self) -> Option<&Self> {
        match self {
            Self::Origin(parent) => Some(&parent.parent),
            Self::Legacy { .. } => None,
            #[cfg(test)]
            Self::Admitted(_) => None,
        }
    }

    /// Read-only header metadata belongs to the same captured root resource.
    /// Derived origins never borrow a selected successor's title catalogue.
    pub(super) fn root_get<'a>(&self, shell: &'a Shell) -> Option<&'a ConversationArea> {
        match self {
            Self::Origin(parent) => parent.parent.root_get(shell),
            Self::Legacy { .. } => self.get(shell),
            #[cfg(test)]
            Self::Admitted(_) => self.get(shell),
        }
    }

    /// Legacy accounting stays legacy. Scoped failures never consult that path.
    pub(super) fn origin_height(&self, shell: &Shell) -> Option<usize> {
        match self {
            Self::Origin(parent) => parent.parent.get(shell).map(ConversationArea::head),
            Self::Legacy { .. } => Some(shell.areas.conversation.head()),
            #[cfg(test)]
            Self::Admitted(_) => None,
        }
    }

    /// Borrow the captured resource from Shell; the output never borrows this carrier.
    pub(crate) fn get<'a>(&self, shell: &'a Shell) -> Option<&'a ConversationArea> {
        match self {
            Self::Legacy { read, .. } => read(shell),
            Self::Origin(parent) => parent.parent.get(shell)?.origin.area(),
            #[cfg(test)]
            Self::Admitted(seat) => seat.owner.get(shell),
        }
    }

    /// Scoped local input retains the checked visible/focused predicate at every level.
    pub(crate) fn get_mut<'a>(&self, shell: &'a mut Shell) -> Option<&'a mut ConversationArea> {
        match self {
            Self::Legacy { write, .. } => write(shell),
            Self::Origin(parent) => parent.parent.get_mut(shell)?.origin.area_mut(),
            #[cfg(test)]
            Self::Admitted(seat) => seat.owner.get_mut(shell),
        }
    }

    /// A clone of one capture-time presentation scope, shared by rows and sheets.
    pub(super) fn render_scope(&self) -> Option<RenderScope> {
        match self {
            Self::Legacy { .. } => None,
            Self::Origin(origin) => origin.scope.clone(),
            #[cfg(test)]
            Self::Admitted(seat) => Some(seat.scope.clone()),
        }
    }

    /// The old alias transport authorizes no admitted operation. W12's actual
    /// cutover must supply the original guarded binding/ticket path separately.
    pub(super) fn legacy_target<'a>(&self, shell: &'a Shell) -> Option<&'a Shown> {
        match self {
            Self::Legacy { read, .. } => read(shell)?.shown.as_ref(),
            Self::Origin(_) => None,
            #[cfg(test)]
            Self::Admitted(_) => None,
        }
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a seat's reader may find no state; the session area's always has one"
)]
const fn session(shell: &Shell) -> Option<&ConversationArea> {
    Some(&shell.areas.conversation)
}
#[expect(
    clippy::unnecessary_wraps,
    reason = "a seat's reader may find no state; the session area's always has one"
)]
const fn session_mut(shell: &mut Shell) -> Option<&mut ConversationArea> {
    Some(&mut shell.areas.conversation)
}
fn origin(shell: &Shell) -> Option<&ConversationArea> {
    shell.areas.conversation.origin.area()
}
fn origin_mut(shell: &mut Shell) -> Option<&mut ConversationArea> {
    shell.areas.conversation.origin.area_mut()
}
