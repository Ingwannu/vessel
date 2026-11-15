//! Node-answer and comparable presentation cache. This module never draws or
//! dispatches; View remains the authority for rows, permissions and correlation.
use vessel_app_kit::ui::SharedString;
use vessel_app_link::schedules::{Answer, ListValue, Message, Value};
use vessel_app_view::schedules::{
    clock::Clock,
    load::Failure,
    page::{Ask, Body, Page, Snapshot},
    presentation::{Notice, PreparedRow},
};

#[derive(Debug)]
pub(super) struct Data {
    pub(super) page: Page,
    pub(super) rows: Vec<PreparedRow>,
    pub(super) host_errors: Vec<SharedString>,
    pub(super) blocking: Vec<Notice>,
    pub(super) local: Option<Notice>,
    pub(super) body: Body,
    pub(super) clock: Clock,
}
impl Default for Data {
    fn default() -> Self {
        Self {
            page: Page::default(),
            rows: Vec::new(),
            host_errors: Vec::new(),
            blocking: Vec::new(),
            local: None,
            body: Body::Loading,
            clock: Clock::unknown(),
        }
    }
}
impl Data {
    pub(crate) fn message(&mut self, message: Message) {
        match message {
            Message::Connected(key) => self.page.connected(&key),
            Message::Replied(answer) => self.answered(answer),
        }
    }

    pub(crate) fn answered(&mut self, answer: Answer) {
        let ask = answer.request.ask;
        let result = answer.result.map_err(|error| error.failure(ask));
        match ask {
            Ask::List => self.page.listed(
                &answer.request,
                result.and_then(|value| match value {
                    Value::List(list) => Ok(snapshot(*list)),
                    Value::Written(_) => Err(Failure::Unexpected),
                }),
            ),
            Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => {
                self.page.written(
                    &answer.request,
                    result.and_then(|value| match value {
                        Value::Written(written) => Ok(written),
                        Value::List(_) => Err(Failure::Unconfirmed),
                    }),
                );
            }
        }
    }
}
fn snapshot(list: ListValue) -> Snapshot {
    Snapshot {
        schedules: list.schedules,
        read_at: list.read_at,
        projects: list.projects.map_err(|error| error.failure(Ask::List)),
        worktrees: list.worktrees.map_err(|error| error.failure(Ask::List)),
    }
}
