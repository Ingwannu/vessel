//! The reply is checked at this application edge. A refused or ambiguous
//! write settles the original row ticket without replaying the operation.
use vessel_app_kit::ui::SharedString;
use vessel_app_link::schedules::{Answer, Value};
use vessel_app_view::schedules::{
    load::Failure,
    page::{Ask, Body, Page},
    presentation::{Notice, PreparedRow},
};

pub(super) struct Data {
    pub(super) page: Page,
    pub(super) rows: Vec<PreparedRow>,
    pub(super) host_errors: Vec<SharedString>,
    pub(super) blocking: Vec<Notice>,
    pub(super) local: Option<Notice>,
    pub(super) body: Body,
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
        }
    }
}

impl Data {
    pub(super) fn answered(&mut self, answer: Answer) {
        let (request, result) = answer.settle();
        match request.ask {
            Ask::List => self.page.listed(
                &request,
                result.and_then(|value| match value {
                    Value::Listed(snapshot) => Ok(*snapshot),
                    Value::Written(_) => Err(Failure::Unexpected),
                }),
            ),
            Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => {
                self.page.written(
                    &request,
                    result.and_then(|value| match value {
                        Value::Written(written) => Ok(written),
                        Value::Listed(_) => Err(Failure::Unconfirmed),
                    }),
                );
            }
        }
    }
}
