//! Which viewer of a terminal answers its queries (ADR 0073): one call per
//! viewer, so a program asking where its cursor is gets one answer, from
//! the person typing into it, and never from a viewer only watching.
//!
//! [Decision Log]
//! - Purpose: several apps can follow one terminal. If each answered, the
//!   program would read a cursor report once per viewer, typed into it.
//! - Constraint: Paseo answers in its daemon, which has no viewers; this
//!   node does not answer (no new call), so a viewer must.
//! - Choice: a viewer that has focus and has typed answers (the rule that
//!   already lets a view resize, `ResizeRule`); until some viewer
//!   qualifies, the view that created the terminal answers, even with
//!   focus lost and nothing typed, so a program started from it is not
//!   left waiting (nvim asks at once). Every other viewer is `Silent` and
//!   applies its bytes with `TerminalModel::replay`.
//! - Cost: an attach, or a terminal whose creating view has closed, with
//!   nobody qualified, answers nothing and the program waits, a stated
//!   difference from Paseo. Two viewers that both have focus and have
//!   typed both answer.

/// How this view came to the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// This view created it.
    Opened,
    /// This view attached to one that was running.
    Attached,
}

/// Whether this view has the keyboard's focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// Held.
    Held,
    /// Lost.
    Lost,
}

/// Whether the person has typed in this view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typed {
    /// Not yet.
    Never,
    /// At least once.
    Once,
}

/// Whether some other viewer already has focus and has typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Others {
    /// None does.
    NoneQualify,
    /// One does.
    OneQualifies,
}

/// Which model call this viewer makes with live bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryDuty {
    /// `TerminalModel::feed`, sending its replies.
    Answer,
    /// `TerminalModel::replay`.
    Silent,
}

/// This viewer's duty.
#[must_use]
pub const fn query_duty(origin: Origin, focus: Focus, typed: Typed, others: Others) -> QueryDuty {
    match (focus, typed, origin, others) {
        (Focus::Held, Typed::Once, _, _) | (_, _, Origin::Opened, Others::NoneQualify) => {
            QueryDuty::Answer
        }
        (Focus::Lost, _, Origin::Attached, _)
        | (Focus::Lost, _, Origin::Opened, Others::OneQualifies)
        | (Focus::Held, Typed::Never, Origin::Attached, _)
        | (Focus::Held, Typed::Never, Origin::Opened, Others::OneQualifies) => QueryDuty::Silent,
    }
}
