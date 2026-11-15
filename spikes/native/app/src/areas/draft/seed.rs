//! Top-level workspace intake precedes navigation and never overwrites a draft.

#[cfg(test)]
mod tests;

use crate::{
    routes::{self, Route},
    shell::Shell,
};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::machine_book::Name;
use vessel_app_view::{
    composer::{context::ContextText, drafts::DraftKey},
    creation::{
        DraftDestination, DraftSeed, Launch, NewWorkspaceSeed, PayloadPresence, SeedPlace,
        SeedProblem, WorkspaceForm,
    },
};

impl Shell {
    /// Header/command-center entry: no workspace or layout id exists yet.
    pub(crate) fn open_new_workspace(
        &mut self,
        seed: NewWorkspaceSeed,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), SeedProblem> {
        self.open_workspace_seed(
            DraftSeed {
                destination: seed.destination,
                place: SeedPlace::NewWorkspace(seed.project),
                launch: seed.launch,
                text: ContextText::parse("")?,
                history: None,
            },
            window,
            cx,
        )
    }

    /// Fork/create producers move canonical payload to its owner before opening.
    pub(crate) fn open_workspace_seed(
        &mut self,
        seed: DraftSeed,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), SeedProblem> {
        let SeedPlace::NewWorkspace(project) = seed.place else {
            return Err(SeedProblem::TargetMismatch);
        };
        if self.draft_has_payload(cx) || self.areas.draft.creation.registered.is_some() {
            return Err(SeedProblem::Busy);
        }
        if !self.fleet.machines.iter().any(|machine| {
            machine.name == seed.destination.machine.as_str()
                && machine.node == Some(seed.destination.node)
        }) {
            return Err(SeedProblem::DestinationChanged);
        }
        let folder = project
            .as_ref()
            .map(|project| project.root.to_str().ok_or(SeedProblem::PlacementGone))
            .transpose()?;
        if let Some(history) = seed.history {
            self.composer_seed_history(DraftKey::New, history, window, cx)?;
        }
        self.choose_machine(seed.destination.machine.as_str().to_owned(), window, cx);
        self.forget_place();
        if let Some(folder) = folder {
            folder.clone_into(&mut self.areas.draft.folder);
        }
        self.areas
            .draft
            .composer
            .set_text(seed.text.as_str(), window, cx);
        // A seed writes the field without an edit event, just like the scoped
        // editor. Reuse the existing text owner before opening the screen.
        self.new_chat_kept(cx);
        self.areas.draft.creation.workspace = Some(WorkspaceForm {
            destination: seed.destination,
            project,
            launch: seed.launch,
        });
        self.areas.draft.creation.plan = None;
        self.areas.draft.error = None;
        self.route = Route::NewWorkspace;
        routes::arrive(Route::NewWorkspace, self, window, cx);
        cx.notify();
        Ok(())
    }

    /// The generic route action uses the current host; specific features pass one.
    pub(crate) fn new_workspace_current(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let chosen = if self.route == Route::Session {
            self.areas
                .session
                .open
                .as_ref()
                .map(|open| open.machine.as_str())
        } else {
            self.areas
                .draft
                .machine
                .as_deref()
                .or(self.prefs.machine.as_deref())
        };
        let destination = chosen
            .and_then(|name| {
                self.fleet
                    .machines
                    .iter()
                    .find(|machine| machine.name == name)
            })
            .and_then(|machine| {
                Some(DraftDestination {
                    machine: Name::parse(&machine.name).ok()?,
                    node: machine.node?,
                })
            });
        let result = match destination {
            Some(destination) => {
                self.open_new_workspace(NewWorkspaceSeed::new(destination, None), window, cx)
            }
            None => Err(SeedProblem::DestinationChanged),
        };
        if let Err(problem) = result {
            self.seed_refused(problem, cx);
        }
    }

    fn seed_refused(&mut self, problem: SeedProblem, cx: &mut Context<'_, Self>) {
        let words = self.words.creation();
        let text = match problem {
            SeedProblem::Busy => words.seed_busy(),
            SeedProblem::DestinationChanged => words.offline(),
            SeedProblem::PlacementGone | SeedProblem::TargetMismatch | SeedProblem::Context(_) => {
                words.refused()
            }
        }
        .to_owned();
        self.toast(text, vessel_app_view::toasts::ToastTone::Error, cx);
    }

    pub(super) fn draft_heading(&self) -> String {
        if self.route == Route::NewWorkspace {
            self.words.creation().new_workspace().to_owned()
        } else {
            self.t.welcome.to_owned()
        }
    }

    /// Until the reviewed registry driver lands, refuse instead of creating a chat.
    pub(super) fn submit_workspace_form(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        let target = self
            .areas
            .draft
            .creation
            .workspace
            .as_ref()
            .map_or(Launch::Chat, |form| form.launch.clone());
        let payload = if self.draft_has_files() {
            PayloadPresence::Present
        } else {
            PayloadPresence::Empty
        };
        let profiles = vessel_proto::terminal_profiles::Profiles::new(Vec::new());
        match profiles
            .map_err(vessel_app_view::creation::LaunchProblem::Profile)
            .and_then(|profiles| target.prepare(text, payload, &profiles))
        {
            Ok(plan) => {
                self.areas.draft.creation.plan = Some(plan);
                self.areas.draft.error = Some(self.words.creation().unsupported().to_owned());
            }
            Err(problem) => {
                self.areas.draft.creation.plan = None;
                self.areas.draft.error = Some(
                    match problem {
                        vessel_app_view::creation::LaunchProblem::MissingProfile
                        | vessel_app_view::creation::LaunchProblem::Profile(_)
                        | vessel_app_view::creation::LaunchProblem::ChatPayload => {
                            self.words.creation().refused()
                        }
                    }
                    .to_owned(),
                );
            }
        }
        cx.notify();
    }
}
