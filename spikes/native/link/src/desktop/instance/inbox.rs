//! A window retains the latest intent of each kind until that owner's handler is ready.

use super::{Launch, ProjectDirectory};
use vessel_app_view::desktop::NavigationTarget;

/// Project readiness must not block or erase independently received session links.
#[derive(Debug, Default)]
pub struct Inbox {
    session: Option<NavigationTarget>,
    project: Option<ProjectDirectory>,
}

impl Inbox {
    /// Replace only the same kind of intent; opening a window itself needs no queue.
    pub fn push(&mut self, intent: Launch) {
        match intent {
            Launch::Session(target) => self.session = Some(target),
            Launch::Project(directory) => self.project = Some(directory),
            Launch::NewWindow => {}
        }
    }

    /// The authenticated session owner is invoked only when its handler is installed.
    #[must_use]
    pub fn take_session(&mut self) -> Option<NavigationTarget> {
        self.session.take()
    }

    /// The managed-local project owner decides when this absolute directory is ready.
    #[must_use]
    pub fn take_project(&mut self) -> Option<ProjectDirectory> {
        self.project.take()
    }

    /// A temporarily unready handler cannot replace a newer arriving directory.
    pub fn defer_project(&mut self, previous: ProjectDirectory) {
        if self.project.is_none() {
            self.project = Some(previous);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use vessel_proto::{NodeId, SessionId};

    #[test]
    fn independent_readiness_never_loses_a_project_when_a_session_link_arrives() {
        let project = ProjectDirectory::new(PathBuf::from(if cfg!(windows) {
            r"C:\project"
        } else {
            "/project"
        }))
        .unwrap();
        let session = NavigationTarget::Session {
            node: NodeId::random(),
            session: SessionId::random(),
        };
        let mut inbox = Inbox::default();
        inbox.push(Launch::Project(project.clone()));
        inbox.push(Launch::Session(session));
        assert_eq!(inbox.take_session(), Some(session));
        assert_eq!(inbox.take_session(), None);
        assert_eq!(inbox.take_project(), Some(project));
    }

    #[test]
    fn replacement_is_bounded_to_the_latest_same_kind_and_new_window_erases_nothing() {
        let mut inbox = Inbox::default();
        let first = NavigationTarget::Session {
            node: NodeId::random(),
            session: SessionId::random(),
        };
        let latest = NavigationTarget::Session {
            node: NodeId::random(),
            session: SessionId::random(),
        };
        inbox.push(Launch::Session(first));
        inbox.push(Launch::Session(latest));
        inbox.push(Launch::NewWindow);
        assert_eq!(inbox.take_session(), Some(latest));
        assert_eq!(inbox.take_project(), None);
    }

    #[test]
    fn deferred_old_directory_cannot_overwrite_a_newer_request_during_delivery() {
        let prefix = if cfg!(windows) { r"C:\" } else { "/" };
        let old = ProjectDirectory::new(PathBuf::from(prefix).join("old")).unwrap();
        let latest = ProjectDirectory::new(PathBuf::from(prefix).join("latest")).unwrap();
        let mut inbox = Inbox::default();
        inbox.push(Launch::Project(old.clone()));
        let captured = inbox.take_project().unwrap();
        inbox.push(Launch::Project(latest.clone()));
        inbox.defer_project(captured);
        assert_eq!(inbox.take_project(), Some(latest));
        inbox.defer_project(old.clone());
        assert_eq!(inbox.take_project(), Some(old));
    }
}
