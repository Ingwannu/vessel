//! Apply C1 results only to the machine snapshot that admitted their request.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{Answer, project_requests::Response};
use vessel_app_view::workspace::projects::actions::ProjectSettlement;

impl Shell {
    pub(crate) fn selected_project_answer(
        &mut self,
        response: Response,
        cx: &mut Context<'_, Self>,
    ) {
        let host = response.context.machine().as_str();
        if self.project_connection(host).as_ref() != Some(&response.context) {
            // Retired writes still settle tracking, but cannot hydrate a new
            // admission or close a newer draft. Refresh is an explicit read.
            if let Answer::ProjectChanged { key, .. } = response.answer {
                let settled = self
                    .areas
                    .workspace
                    .page
                    .actions
                    .finish(&key, ProjectSettlement::Failed);
                if settled == vessel_app_view::workspace::projects::actions::ProjectReply::Current {
                    self.areas.workspace.page.error =
                        Some(self.words.projects().outcome_unknown().to_owned());
                }
            }
            return;
        }
        let _changed = self.workspace_answer(host, response.answer, cx);
    }

    pub(super) fn project_cache_current(&self, host: &vessel_app_view::machine_book::Name) -> bool {
        let current = self.project_connection(host.as_str());
        current.is_some() && self.areas.workspace.project_reads.context(host) == current.as_ref()
    }
}
