//! One area command fan-in; row and editor work share the same actor/pool.
use super::{Answer, Message, RowRequest, ScheduleError};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::schedules::editor::io;
use vessel_app_view::schedules::history::reader;

/// Node-backed schedules operations; no extra root dispatcher or request pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Existing list/pause/resume/run/delete envelope, unchanged.
    Row(RowRequest),
    /// Project/model/preview/create/update from an explicit form visit.
    Form(io::Request),
    /// Read-only run history from the explicitly selected physical machine.
    History(reader::Request),
    /// Metadata for a run's actual session; this does not open a live panel.
    Lookup(reader::Lookup),
}
impl From<RowRequest> for Request {
    fn from(value: RowRequest) -> Self {
        Self::Row(value)
    }
}
impl From<io::Request> for Request {
    fn from(value: io::Request) -> Self {
        Self::Form(value)
    }
}
impl From<reader::Request> for Request {
    fn from(value: reader::Request) -> Self {
        Self::History(value)
    }
}
impl From<reader::Lookup> for Request {
    fn from(value: reader::Lookup) -> Self {
        Self::Lookup(value)
    }
}
impl Request {
    /// Every family is fenced by its original physical admission.
    #[must_use]
    pub const fn connection(&self) -> &ConnectionContext {
        match self {
            Self::Row(request) => request.connection(),
            Self::Form(request) => request.connection(),
            Self::History(request) => request.connection(),
            Self::Lookup(request) => request.connection(),
        }
    }
    /// Unsent/browser/deadline refusals settle the original family and ticket.
    #[must_use]
    pub fn refused(self, error: ScheduleError) -> Message {
        match self {
            Self::Row(request) => Message::Replied(Answer {
                request,
                result: Err(error),
            }),
            Self::Form(request) => Message::FormReplied(FormAnswer {
                request,
                result: Err(error),
            }),
            Self::History(request) => Message::HistoryReplied(HistoryAnswer {
                request,
                result: Err(error),
            }),
            Self::Lookup(request) => Message::SessionReplied(SessionAnswer {
                request,
                result: Err(error),
            }),
        }
    }
}
/// Editor outcome; no protocol/client errors are erased into Display strings.
#[derive(Debug)]
pub struct FormAnswer {
    /// Exact context/ticket/operation that was dispatched.
    pub request: io::Request,
    /// Node payload or a source-preserving closed failure.
    pub result: Result<io::Value, ScheduleError>,
}

/// Run-list outcome retains its original resource/admission/ticket on all paths.
#[derive(Debug)]
pub struct HistoryAnswer {
    /// Exact selected schedule request, including its original admission.
    pub request: reader::Request,
    /// Node-authored list or a source-preserving typed failure.
    pub result: Result<vessel_proto::automation::RunList, ScheduleError>,
}

/// Original run/session lookup outcome; never an instruction to open a panel.
#[derive(Debug)]
pub struct SessionAnswer {
    /// Full original admission/visit/run/session/ticket on success and refusal.
    pub request: reader::Lookup,
    /// Authoritative metadata, still requiring the live owner's admission.
    pub result: Result<vessel_proto::SessionInfo, ScheduleError>,
}
