//! What each page of the Add project sheet lists, Paseo's
//! `add-project-flow.tsx` rows (pinned 0.15.4; the page's
//! `add-project/rows.ts`), host as machine. Rows are typed values; their
//! words are the language's (`text::workspace::WorkspaceWords`).

use std::collections::HashMap;

use super::model::{CloneProtocol, Machine, Origin, Page, PageId, PageKind, Repository};
use super::options::{
    Method, MethodNote, base_name, clone_locations, filter_machines, methods, suggested_parents,
};
use super::repositories::github_choices;
use super::paths::{DirectoryOption, directory_options, shorten_path};

/// The mark a row draws (Paseo's lucide icons; `Drive` is its `HardDrive`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowMark {
    /// A machine.
    Machine,
    /// Add a machine.
    Plus,
    /// Search for directory.
    Search,
    /// Browse.
    FolderOpen,
    /// GitHub's mark.
    Github,
    /// New directory.
    FolderPlus,
    /// A folder.
    Folder,
    /// A clone destination.
    Drive,
}

/// What choosing a row does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowAction {
    /// Go to the machine's methods.
    Machine(String),
    /// Open pairing (no machine is connected).
    AddMachine,
    /// Open a method.
    Method(Method),
    /// Add this folder as a project.
    AddPath(String),
    /// Choose where to clone this repository.
    Repository(Repository),
    /// Clone into this parent.
    Clone(String),
    /// Name a new folder in this parent.
    Parent(String),
}

/// A row's first line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowTitle {
    /// Shown as it is (a machine's label, a path, a repository).
    Text(String),
    /// "Add a machine".
    AddMachine,
    /// A method's name.
    Method(Method),
    /// "{owner/repo} via HTTPS" or "via SSH".
    RepositoryVia(String, CloneProtocol),
}

/// A row's second line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowNote {
    /// Shown as it is (a machine's name, a whole path).
    Text(String),
    /// "No connected machines".
    NoMachines,
    /// A method's line.
    Method(MethodNote),
    /// "Open this path".
    OpenThisPath,
    /// "Use this parent".
    UseThisParent,
    /// "Clone this repository URL".
    CloneThisUrl,
    /// "Clone owner/repo via HTTPS" or "via SSH".
    CloneVia(CloneProtocol),
    /// "Already exists".
    AlreadyExists,
    /// "Parent directory: {parent}".
    ParentDirectory(String),
}

/// One row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Its identity in the list.
    pub id: String,
    /// The first line.
    pub title: RowTitle,
    /// The second line, if any.
    pub note: Option<RowNote>,
    /// Its mark.
    pub mark: RowMark,
    /// It cannot be chosen.
    pub disabled: bool,
    /// What choosing it does.
    pub action: RowAction,
}

/// What the sheet knows for the page shown.
#[derive(Debug, Clone, Copy)]
pub struct Inputs<'a> {
    /// The page shown.
    pub page: &'a Page,
    /// Every machine.
    pub machines: &'a [Machine],
    /// The page's machine.
    pub machine: Option<&'a Machine>,
    /// The machine's projects' roots (plan difference 4).
    pub recent: &'a [String],
    /// The machine's `folder.suggest` answer for the field as it is.
    pub suggested: &'a [String],
    /// The node's `~` form of each answered path.
    pub shown: &'a HashMap<String, String>,
    /// Where the last clone on this machine went.
    pub last_clone_parent: Option<&'a str>,
    /// The machine reads a leading `~` (ADR 0113); an older one is sent
    /// no `~` path.
    pub tilde: bool,
    /// The machine's GitHub search for the field as it is (ADR 0088).
    pub repositories: &'a [Repository],
}

impl Inputs<'_> {
    fn display(&self, path: &str) -> String {
        self.shown
            .get(path)
            .cloned()
            .unwrap_or_else(|| shorten_path(path))
    }

    fn usable(&self, options: Vec<DirectoryOption>) -> Vec<DirectoryOption> {
        options
            .into_iter()
            .filter(|option| self.tilde || !option.path.starts_with('~'))
            .collect()
    }
}

const fn method_mark(method: Method) -> RowMark {
    match method {
        Method::DirectorySearch => RowMark::Search,
        Method::Browse => RowMark::FolderOpen,
        Method::Github => RowMark::Github,
        Method::NewDirectory => RowMark::FolderPlus,
    }
}

/// A method row's identity (the page's ids).
const fn method_id(method: Method) -> &'static str {
    match method {
        Method::DirectorySearch => "directory-search",
        Method::Browse => "browse",
        Method::Github => "github",
        Method::NewDirectory => "new-directory",
    }
}

fn host_rows(inputs: &Inputs<'_>) -> Vec<Row> {
    let mut rows: Vec<Row> = filter_machines(inputs.machines, &inputs.page.field)
        .into_iter()
        .map(|machine| Row {
            id: machine.name.clone(),
            title: RowTitle::Text(machine.label.clone()),
            note: Some(RowNote::Text(machine.name.clone())),
            mark: RowMark::Machine,
            disabled: false,
            action: RowAction::Machine(machine.name.clone()),
        })
        .collect();
    if inputs.machines.is_empty() {
        rows.push(Row {
            id: "add-machine".to_owned(),
            title: RowTitle::AddMachine,
            note: Some(RowNote::NoMachines),
            mark: RowMark::Plus,
            disabled: false,
            action: RowAction::AddMachine,
        });
    }
    rows
}

fn method_rows(machine: &Machine) -> Vec<Row> {
    methods(machine)
        .into_iter()
        .map(|option| Row {
            id: method_id(option.method).to_owned(),
            title: RowTitle::Method(option.method),
            note: Some(RowNote::Method(option.note)),
            mark: method_mark(option.method),
            disabled: option.disabled,
            action: RowAction::Method(option.method),
        })
        .collect()
}

fn directory_rows(inputs: &Inputs<'_>) -> Vec<Row> {
    let options = directory_options(inputs.recent, inputs.suggested, &inputs.page.field);
    inputs
        .usable(options)
        .into_iter()
        .map(|option| {
            let short = inputs.display(&option.path);
            let note = if option.typed {
                Some(RowNote::OpenThisPath)
            } else if short == option.path {
                None
            } else {
                Some(RowNote::Text(option.path.clone()))
            };
            Row {
                id: option.path.clone(),
                title: RowTitle::Text(short),
                note,
                mark: RowMark::Folder,
                disabled: false,
                action: RowAction::AddPath(option.path),
            }
        })
        .collect()
}

/// Paseo's GitHub rows: what was typed (unless the search has it), then
/// the search's, each found row's note its description.
fn repository_rows(query: &str, found: &[Repository]) -> Vec<Row> {
    github_choices(query, found)
        .into_iter()
        .map(|repository| {
            let (title, note) = match (repository.protocol, &repository.origin) {
                (Some(protocol), _) => (
                    RowTitle::RepositoryVia(repository.name_with_owner.clone(), protocol),
                    Some(RowNote::CloneVia(protocol)),
                ),
                (None, Origin::Typed) => (
                    RowTitle::Text(repository.name_with_owner.clone()),
                    Some(RowNote::CloneThisUrl),
                ),
                (None, Origin::Found { description }) => (
                    RowTitle::Text(repository.name_with_owner.clone()),
                    description.clone().map(RowNote::Text),
                ),
            };
            Row {
                id: repository.id.clone(),
                title,
                note,
                mark: RowMark::Github,
                disabled: false,
                action: RowAction::Repository(repository),
            }
        })
        .collect()
}

fn location_rows(inputs: &Inputs<'_>, repository: &Repository) -> Vec<Row> {
    let parents = suggested_parents(inputs.recent);
    let ordered: Vec<String> = match inputs.last_clone_parent {
        Some(last) => std::iter::once(last.to_owned())
            .chain(parents.into_iter().filter(|parent| parent != last))
            .collect(),
        None => parents,
    };
    let filtered: Vec<String> = inputs
        .usable(directory_options(
            &ordered,
            inputs.suggested,
            &inputs.page.field,
        ))
        .into_iter()
        .map(|option| option.path)
        .collect();
    let existing: Vec<String> = inputs
        .recent
        .iter()
        .chain(inputs.suggested)
        .cloned()
        .collect();
    clone_locations(&filtered, base_name(&repository.name_with_owner), &existing)
        .into_iter()
        .map(|location| Row {
            id: location.parent.clone(),
            title: RowTitle::Text(inputs.display(&location.path)),
            note: Some(if location.exists {
                RowNote::AlreadyExists
            } else {
                RowNote::ParentDirectory(location.parent.clone())
            }),
            mark: RowMark::Drive,
            disabled: location.exists,
            action: RowAction::Clone(location.parent),
        })
        .collect()
}

fn parent_rows(inputs: &Inputs<'_>) -> Vec<Row> {
    let options = directory_options(inputs.recent, inputs.suggested, &inputs.page.field);
    inputs
        .usable(options)
        .into_iter()
        .map(|option| Row {
            id: option.path.clone(),
            title: RowTitle::Text(inputs.display(&option.path)),
            note: Some(if option.typed {
                RowNote::UseThisParent
            } else {
                RowNote::Text(option.path.clone())
            }),
            mark: RowMark::Folder,
            disabled: false,
            action: RowAction::Parent(option.path),
        })
        .collect()
}

/// The rows of the page shown.
#[must_use]
pub fn rows(inputs: &Inputs<'_>) -> Vec<Row> {
    match &inputs.page.kind {
        PageKind::Host => host_rows(inputs),
        PageKind::Method { .. } => inputs.machine.map(method_rows).unwrap_or_default(),
        PageKind::DirectorySearch { .. } => directory_rows(inputs),
        PageKind::GithubSearch { .. } => repository_rows(&inputs.page.field, inputs.repositories),
        PageKind::GithubLocation { repository, .. } => location_rows(inputs, repository),
        PageKind::NewDirectoryParent { .. } => parent_rows(inputs),
        PageKind::NewDirectoryName { .. } => Vec::new(),
    }
}

/// Whether the page's field asks the machine for folders.
#[must_use]
pub const fn searches_directories(page: PageId) -> bool {
    matches!(
        page,
        PageId::DirectorySearch | PageId::GithubLocation | PageId::NewDirectoryParent
    )
}

/// The words when a page lists nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Empty {
    /// "No connected machines".
    NoMachines,
    /// "Enter a GitHub URL or owner/repo".
    EnterRepository,
    /// "Update the machine to use Add Project."
    UpdateMachine,
    /// "No matching options".
    NoMatches,
}

/// What a page says when it lists nothing.
#[must_use]
pub fn empty(page: PageId, machine: Option<&Machine>) -> Empty {
    match page {
        PageId::Host => Empty::NoMachines,
        PageId::GithubSearch => Empty::EnterRepository,
        PageId::Method if machine.is_some_and(|machine| !machine.can_add_project) => {
            Empty::UpdateMachine
        }
        PageId::Method
        | PageId::DirectorySearch
        | PageId::GithubLocation
        | PageId::NewDirectoryParent
        | PageId::NewDirectoryName => Empty::NoMatches,
    }
}
