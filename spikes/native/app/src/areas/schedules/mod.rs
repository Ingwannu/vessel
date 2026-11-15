//! Schedule reply boundary. The view owns all list/permission/pending decisions;
//! drawing/route entry are a following part of the list slice, not fake data.
use vessel_app_link::schedules::{Answer, ListValue, Value};
use vessel_app_view::schedules::{
    load::Failure,
    page::{Ask, Page, Snapshot},
};

#[derive(Debug, Default)]
pub(crate) struct Area {
    pub(crate) page: Page,
}
impl Area {
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
#[cfg(test)]
mod tests;
