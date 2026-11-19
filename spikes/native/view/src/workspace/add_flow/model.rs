//! Add project's page stack and its edits, Paseo's
//! `add-project-flow/model.ts` (pinned 0.15.4; the page's
//! `add-project/model.ts` and `editing.ts`), host as machine.
//!
//! The pages' common parts (the field's text, the active row, the error,
//! the progress) are one struct; what differs by page is its [`PageKind`].
//! The shown page is held apart from the pages under it, so there is always
//! one to show.

use std::collections::HashMap;

/// A machine the sheet may add to, and which ways it offers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per way of adding, Paseo's host capabilities as the page's AddProjectMachine has them; each is independent"
)]
pub struct Machine {
    /// The machine's name in the book (its key).
    pub name: String,
    /// What it is called on screen.
    pub label: String,
    /// It adds projects (every machine with projects does here).
    pub can_add_project: bool,
    /// The app can show its own folder chooser for it: only the machine
    /// the app runs on.
    pub can_browse: bool,
    /// Its Git can clone.
    pub can_clone: bool,
    /// Its GitHub repositories can be searched (TRACKER 440; none yet).
    pub can_search_github: bool,
    /// It has `project.create_directory` (ADR 0113).
    pub can_create_directory: bool,
}

/// How an `owner/repo` row is cloned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CloneProtocol {
    /// `https://github.com/owner/repo.git`.
    Https,
    /// `git@github.com:owner/repo.git`.
    Ssh,
}

/// A repository to clone, typed by hand (search rows wait on TRACKER 440).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    /// The row's identity, also the destination draft's.
    pub id: String,
    /// `owner/name`, or the remote's last segment.
    pub name_with_owner: String,
    /// What was typed: a whole remote, or `owner/repo`.
    pub clone_url: String,
    /// For `owner/repo`: the transport to make a remote with.
    pub protocol: Option<CloneProtocol>,
}

/// What differs between pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageKind {
    /// Choose machine.
    Host,
    /// The machine's methods.
    Method {
        /// The machine.
        machine: String,
    },
    /// Search for directory.
    DirectorySearch {
        /// The machine.
        machine: String,
    },
    /// Clone from GitHub: the repository.
    GithubSearch {
        /// The machine.
        machine: String,
    },
    /// Clone from GitHub: the destination.
    GithubLocation {
        /// The machine.
        machine: String,
        /// The repository chosen.
        repository: Repository,
    },
    /// New directory: its parent.
    NewDirectoryParent {
        /// The machine.
        machine: String,
    },
    /// New directory: its name.
    NewDirectoryName {
        /// The machine.
        machine: String,
        /// The parent chosen.
        parent: String,
    },
}

/// A page's kind without its data, for words and for checking which page
/// a call's answer belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageId {
    /// [`PageKind::Host`].
    Host,
    /// [`PageKind::Method`].
    Method,
    /// [`PageKind::DirectorySearch`].
    DirectorySearch,
    /// [`PageKind::GithubSearch`].
    GithubSearch,
    /// [`PageKind::GithubLocation`].
    GithubLocation,
    /// [`PageKind::NewDirectoryParent`].
    NewDirectoryParent,
    /// [`PageKind::NewDirectoryName`].
    NewDirectoryName,
}

impl PageKind {
    /// The page's id.
    #[must_use]
    pub const fn id(&self) -> PageId {
        match self {
            Self::Host => PageId::Host,
            Self::Method { .. } => PageId::Method,
            Self::DirectorySearch { .. } => PageId::DirectorySearch,
            Self::GithubSearch { .. } => PageId::GithubSearch,
            Self::GithubLocation { .. } => PageId::GithubLocation,
            Self::NewDirectoryParent { .. } => PageId::NewDirectoryParent,
            Self::NewDirectoryName { .. } => PageId::NewDirectoryName,
        }
    }

    /// The machine the page is on; none on the machine page.
    #[must_use]
    pub fn machine(&self) -> Option<&str> {
        match self {
            Self::Host => None,
            Self::Method { machine }
            | Self::DirectorySearch { machine }
            | Self::GithubSearch { machine }
            | Self::GithubLocation { machine, .. }
            | Self::NewDirectoryParent { machine }
            | Self::NewDirectoryName { machine, .. } => Some(machine.as_str()),
        }
    }
}

/// One page of the sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Which page, and its data.
    pub kind: PageKind,
    /// The field's text: the query, or on the name page the name; the
    /// method page has no field.
    pub field: String,
    /// The active row.
    pub active: usize,
    /// The last call's failure, in the node's words.
    pub error: Option<String>,
    /// A call of this page is running.
    pub submitting: bool,
}

impl Page {
    fn new(kind: PageKind) -> Self {
        Self {
            kind,
            field: String::new(),
            active: 0,
            error: None,
            submitting: false,
        }
    }
}

/// Which way the keyboard moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Down.
    Next,
    /// Up.
    Previous,
}

/// The sheet: its machines, the page shown over the pages under it, and
/// the drafts kept across Back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    /// The machines it may add to.
    pub machines: Vec<Machine>,
    under: Vec<Page>,
    shown: Page,
    /// A name typed under a parent, by machine and parent.
    name_drafts: HashMap<(String, String), String>,
    /// A destination's field and row, by machine and repository.
    location_drafts: HashMap<(String, String), (String, usize)>,
}

/// The machine the sheet starts on: the preferred one, else the only one.
fn initial<'a>(machines: &'a [Machine], preferred: Option<&str>) -> Option<&'a Machine> {
    preferred
        .and_then(|name| machines.iter().find(|machine| machine.name == name))
        .or(match machines {
            [only] => Some(only),
            _ => None,
        })
}

fn method_page(machine: &str) -> Page {
    Page::new(PageKind::Method {
        machine: machine.to_owned(),
    })
}

impl Flow {
    /// A new sheet: on the preferred or only machine's methods, else on the
    /// machine page (so Back from the methods closes).
    #[must_use]
    pub fn open(machines: Vec<Machine>, preferred: Option<&str>) -> Self {
        let shown = initial(&machines, preferred).map_or_else(
            || Page::new(PageKind::Host),
            |machine| method_page(&machine.name),
        );
        Self {
            machines,
            under: Vec::new(),
            shown,
            name_drafts: HashMap::new(),
            location_drafts: HashMap::new(),
        }
    }

    /// Machines arrived or changed: while the machine page is the only
    /// page, a preferred or only machine skips it.
    pub fn set_machines(&mut self, machines: Vec<Machine>, preferred: Option<&str>) {
        if self.under.is_empty()
            && self.shown.kind == PageKind::Host
            && let Some(machine) = initial(&machines, preferred)
        {
            self.shown = method_page(&machine.name);
        }
        self.machines = machines;
    }

    /// The page shown.
    #[must_use]
    pub const fn page(&self) -> &Page {
        &self.shown
    }

    /// How many pages are open, the shown one with them.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.under.len().saturating_add(1)
    }

    /// The machine of the page shown.
    #[must_use]
    pub fn machine(&self) -> Option<&Machine> {
        let name = self.shown.kind.machine()?;
        self.machines.iter().find(|machine| machine.name == name)
    }

    fn push(&mut self, page: Page) {
        let below = std::mem::replace(&mut self.shown, page);
        self.under.push(below);
    }

    /// One page back; `false` on the first page (the sheet closes).
    pub fn back(&mut self) -> bool {
        match self.under.pop() {
            Some(page) => {
                self.shown = page;
                true
            }
            None => false,
        }
    }

    /// The machine page's row chosen.
    pub fn choose_machine(&mut self, machine: &str) {
        self.push(method_page(machine));
    }

    /// Search for directory.
    pub fn open_directory_search(&mut self, machine: &str) {
        self.push(Page::new(PageKind::DirectorySearch {
            machine: machine.to_owned(),
        }));
    }

    /// Clone from GitHub.
    pub fn open_github_search(&mut self, machine: &str) {
        self.push(Page::new(PageKind::GithubSearch {
            machine: machine.to_owned(),
        }));
    }

    /// The destination page for `repository`, with what was typed there
    /// the last time it was open.
    pub fn open_github_location(&mut self, machine: &str, repository: Repository) {
        let key = (machine.to_owned(), repository.id.clone());
        let (field, active) = self.location_drafts.get(&key).cloned().unwrap_or_default();
        self.push(Page {
            field,
            active,
            ..Page::new(PageKind::GithubLocation {
                machine: machine.to_owned(),
                repository,
            })
        });
    }

    /// New directory.
    pub fn open_new_directory_parent(&mut self, machine: &str) {
        self.push(Page::new(PageKind::NewDirectoryParent {
            machine: machine.to_owned(),
        }));
    }

    /// The name page under `parent`, with the name typed there before.
    pub fn open_new_directory_name(&mut self, machine: &str, parent: &str) {
        let key = (machine.to_owned(), parent.to_owned());
        let field = self.name_drafts.get(&key).cloned().unwrap_or_default();
        self.push(Page {
            field,
            ..Page::new(PageKind::NewDirectoryName {
                machine: machine.to_owned(),
                parent: parent.to_owned(),
            })
        });
    }

    fn keep_location_draft(&mut self) {
        if let PageKind::GithubLocation {
            machine,
            repository,
        } = &self.shown.kind
        {
            drop(self.location_drafts.insert(
                (machine.clone(), repository.id.clone()),
                (self.shown.field.clone(), self.shown.active),
            ));
        }
    }

    /// The field changed: its text, the first row active, no error; a
    /// destination keeps it as its draft, a name as the parent's draft.
    /// The method page has no field.
    pub fn set_field(&mut self, value: &str) {
        if self.shown.kind.id() == PageId::Method {
            return;
        }
        value.clone_into(&mut self.shown.field);
        self.shown.active = 0;
        self.shown.error = None;
        self.keep_location_draft();
        if let PageKind::NewDirectoryName { machine, parent } = &self.shown.kind {
            drop(
                self.name_drafts
                    .insert((machine.clone(), parent.clone()), value.to_owned()),
            );
        }
    }

    /// The active row.
    pub fn set_active(&mut self, active: usize) {
        self.shown.active = active;
        self.keep_location_draft();
    }

    /// A call of the page `id` started, when it is still the page shown.
    pub fn begin(&mut self, id: PageId) {
        if self.shown.kind.id() == id {
            self.shown.submitting = true;
            self.shown.error = None;
        }
    }

    /// A call of the page `id` failed with `message`, when it is still the
    /// page shown.
    pub fn fail(&mut self, id: PageId, message: String) {
        if self.shown.kind.id() == id {
            self.shown.submitting = false;
            self.shown.error = Some(message);
        }
    }
}

/// The next or previous of `count` rows, wrapping.
#[must_use]
pub fn move_index(active: usize, count: usize, direction: Direction) -> usize {
    if count == 0 {
        return 0;
    }
    match direction {
        Direction::Next => match active.checked_add(1) {
            Some(next) if next < count => next,
            Some(_) | None => 0,
        },
        // Paseo's: below the first is the last; past the end is the first.
        Direction::Previous => match active.checked_sub(1) {
            Some(previous) if previous < count => previous,
            Some(_) => 0,
            None => count.saturating_sub(1),
        },
    }
}

/// The next or previous row that can be chosen, wrapping; where it was
/// when none can (the first row when there is none at all).
#[must_use]
pub fn move_selection(active: usize, selectable: &[bool], direction: Direction) -> usize {
    if !selectable.contains(&true) {
        return 0;
    }
    let mut next = active;
    for _ in 0..selectable.len() {
        next = move_index(next, selectable.len(), direction);
        if selectable.get(next) == Some(&true) {
            return next;
        }
    }
    active
}
