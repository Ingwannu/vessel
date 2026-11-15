//! Node-answer and comparable presentation cache. This module never draws or
//! dispatches; View remains the authority for rows, permissions and correlation.
use vessel_app_kit::ui::SharedString;
use vessel_app_link::schedules::{Answer, ListValue, Message, Value, ViewerClockError};
use vessel_app_view::schedules::{
    clock::Clock,
    load::Failure,
    page::{Ask, Body, Page, Snapshot},
    presentation::{Notice, PreparedRow},
};

#[derive(Debug)]
pub(super) struct Data {
    pub(super) page: Page,
    pub(super) editor: vessel_app_view::schedules::editor::io::Mailbox,
    pub(super) rows: Vec<PreparedRow>,
    pub(super) host_errors: Vec<SharedString>,
    pub(super) blocking: Vec<Notice>,
    pub(super) local: Option<Notice>,
    pub(super) body: Body,
    pub(super) clock: Result<Clock, ViewerClockError>,
}
impl Default for Data {
    fn default() -> Self {
        Self {
            page: Page::default(),
            editor: vessel_app_view::schedules::editor::io::Mailbox::default(),
            rows: Vec::new(),
            host_errors: Vec::new(),
            blocking: Vec::new(),
            local: None,
            body: Body::Loading,
            clock: Err(ViewerClockError::Unavailable),
        }
    }
}
impl Data {
    pub(crate) fn message(&mut self, message: Message) -> bool {
        match message {
            Message::Connected(key) => {
                self.page.connected(&key);
                self.editor.connected(&key)
            }
            Message::Replied(answer) => {
                self.answered(answer);
                false
            }
            Message::FormReplied(answer) => {
                let result = answer
                    .result
                    .map_err(|error| error.editor_failure(answer.request.operation()));
                self.editor.settle(&answer.request, result)
            }
        }
    }

    pub(crate) fn answered(&mut self, answer: Answer) {
        let ask = answer.request.ask;
        let result = answer.result.map_err(|error| error.failure(ask));
        match ask {
            Ask::List => match result {
                Ok(Value::List(list)) => {
                    let (snapshot, clock) = snapshot(*list);
                    if self.page.accept_list(&answer.request, Ok(snapshot)) {
                        self.clock = clock;
                    }
                }
                Ok(Value::Written(_)) => {
                    self.page.listed(&answer.request, Err(Failure::Unexpected));
                }
                Err(error) => {
                    self.page.listed(&answer.request, Err(error));
                }
            },
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
fn snapshot(list: ListValue) -> (Snapshot, Result<Clock, ViewerClockError>) {
    let snapshot = Snapshot {
        schedules: list.schedules,
        scopes: list.scopes.map_err(|error| error.failure(Ask::List)),
        read_at: list.read_at,
        projects: list.projects.map_err(|error| error.failure(Ask::List)),
        worktrees: list.worktrees.map_err(|error| error.failure(Ask::List)),
    };
    (snapshot, list.clock)
}
