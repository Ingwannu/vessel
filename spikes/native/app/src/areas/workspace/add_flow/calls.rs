//! The sheet's calls and their answers (split from `add_flow.rs` for the
//! file budget): a clone into the chosen parent, a new directory after
//! Paseo's name check, the node's answer said on its page or the sheet
//! closed on success, and the move a success asks for once a frame has the
//! window.

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::workspace::{Ask, Form, ProjectAsk};
use vessel_app_view::workspace::add_flow::model::{PageId, PageKind};
use vessel_app_view::workspace::add_flow::options::clone_remote;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::registry::{CreateProjectDirectory, DirectoryName};
use vessel_proto::workspace::{CloneProject, ProjectInfo, ProjectState};

use super::After;
use crate::shell::Shell;

/// Paseo's name check before asking: empty, `.`, `..` or a separator.
fn name_is_usable(name: &str) -> bool {
    let trimmed = name.trim();
    !(trimmed.is_empty() || trimmed == "." || trimmed == ".." || trimmed.contains(['/', '\\']))
}

impl Shell {
    pub(super) fn add_flow_clone(
        &mut self,
        machine: &str,
        parent: String,
        cx: &mut Context<'_, Self>,
    ) {
        let words = self.t.lang.add_flow();
        let Some(open) = &mut self.areas.workspace.add_flow.open else {
            return;
        };
        let PageKind::GithubLocation { repository, .. } = &open.flow.page().kind else {
            return;
        };
        let url = match clone_remote(repository).parse() {
            Ok(url) => url,
            Err(refused) => {
                let said = format!("{refused}");
                open.flow.fail(
                    PageId::GithubLocation,
                    if said.is_empty() {
                        words.cannot_clone.to_owned()
                    } else {
                        said
                    },
                );
                cx.notify();
                return;
            }
        };
        open.flow.begin(PageId::GithubLocation);
        let _was = self
            .areas
            .workspace
            .add_flow
            .last_clone_parent
            .insert(machine.to_owned(), parent.clone());
        self.ask(
            machine,
            Ask::Project(ProjectAsk::Clone(CloneProject {
                url,
                parent: parent.into(),
                folder: None,
                name: None,
            })),
        );
        cx.notify();
    }

    /// Enter on the name page: Paseo's check, then the node makes it.
    pub(super) fn add_flow_create(&mut self, cx: &mut Context<'_, Self>) {
        let words = self.t.lang.add_flow();
        let Some(open) = &mut self.areas.workspace.add_flow.open else {
            return;
        };
        let page = open.flow.page();
        let PageKind::NewDirectoryName { machine, parent } = &page.kind else {
            return;
        };
        if page.submitting {
            return;
        }
        let (machine, parent) = (machine.clone(), parent.clone());
        let name = page.field.trim().to_owned();
        let parsed = name_is_usable(&name)
            .then(|| DirectoryName::parse(&name).ok())
            .flatten();
        let Some(name) = parsed else {
            open.flow
                .fail(PageId::NewDirectoryName, words.enter_name.to_owned());
            cx.notify();
            return;
        };
        open.flow.begin(PageId::NewDirectoryName);
        self.ask(
            &machine,
            Ask::Project(ProjectAsk::CreateDirectory(CreateProjectDirectory {
                parent: parent.into(),
                name,
            })),
        );
        cx.notify();
    }

    /// The node answered the sheet's add, clone or new directory.
    pub(in crate::areas::workspace) fn took_project(
        &mut self,
        machine: &str,
        form: Form,
        result: Result<ProjectInfo, Problem>,
    ) {
        let words = self.t.lang.add_flow();
        let page_words = self.words();
        if let Ok(project) = &result
            && let Some(view) = self.areas.workspace.projects.get_mut(machine)
        {
            view.took(project.clone());
        }
        let sheet = &mut self.areas.workspace.add_flow;
        let Some(open) = &mut sheet.open else {
            return;
        };
        let page = match form {
            Form::Folder => PageId::DirectorySearch,
            Form::Clone => PageId::GithubLocation,
            Form::Directory => PageId::NewDirectoryName,
            Form::New => return,
        };
        match result {
            Ok(project) => {
                sheet.open = None;
                sheet.after = Some(if project.state == ProjectState::Ready {
                    After::Chat {
                        machine: machine.to_owned(),
                        project: project.id,
                    }
                } else {
                    After::Projects(machine.to_owned())
                });
            }
            Err(problem) => {
                let said = page_words.problem(&problem);
                let message = match form {
                    Form::Folder if said.contains("does not exist") => words.not_found.to_owned(),
                    Form::Folder | Form::New => words.cannot_add.to_owned(),
                    Form::Clone if said.is_empty() => words.cannot_clone.to_owned(),
                    Form::Directory if said.is_empty() => words.cannot_create.to_owned(),
                    Form::Clone | Form::Directory => said,
                };
                open.flow.fail(page, message);
            }
        }
    }

    /// The move a success asked for, now that there is a window; the keys
    /// go back where they were before the sheet first.
    pub(super) fn add_flow_after(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let sheet = &mut self.areas.workspace.add_flow;
        let Some(after) = sheet.after.take() else {
            return;
        };
        sheet.field.clear(window, cx);
        let owner = sheet.field.focus_handle(cx);
        sheet.prior.restore_if_owned(&owner, window, cx);
        match after {
            After::Chat { machine, project } => self.chat_in_project(machine, project, window, cx),
            After::Projects(machine) => {
                self.areas.settings.machine = Some(machine);
                self.open_settings(
                    Some(vessel_app_view::settings::Section::Projects),
                    window,
                    cx,
                );
            }
        }
        cx.notify();
    }
}
