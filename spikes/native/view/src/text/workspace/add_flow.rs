//! Add project's words (TRACKER 234), in English and Korean: Paseo's
//! English (`add-project-flow.tsx`, its `options.ts`, pinned 0.15.4), host
//! as machine, the page's words; one table per language so a word missing
//! in one does not compile. A word with a value in it is a template whose
//! `{}` takes the value, so each language puts it where it reads.

use crate::workspace::add_flow::model::{CloneProtocol, PageId};
use crate::workspace::add_flow::options::{Method, MethodNote};
use crate::workspace::add_flow::rows::{Empty, RowNote, RowTitle};

/// The file manager Browse opens (plan difference 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManager {
    /// macOS.
    Finder,
    /// Windows.
    FileExplorer,
    /// Linux and the rest.
    Other,
}

/// One language's Add project words.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the words it holds")]
pub struct AddFlowWords {
    pub choose_machine: &'static str,
    pub add_project: &'static str,
    pub search_for_directory: &'static str,
    pub browse: &'static str,
    pub clone_from_github: &'static str,
    pub new_directory: &'static str,
    pub choose_destination: &'static str,
    pub choose_parent: &'static str,
    pub name_directory: &'static str,
    pub search_machines: &'static str,
    pub search_directories: &'static str,
    pub search_repository: &'static str,
    pub search_parents: &'static str,
    pub directory_name: &'static str,
    pub adding: &'static str,
    pub cloning: &'static str,
    pub creating: &'static str,
    pub loading: &'static str,
    pub no_machines: &'static str,
    pub enter_repository: &'static str,
    pub update_machine: &'static str,
    pub no_matches: &'static str,
    pub add_machine: &'static str,
    pub open_this_path: &'static str,
    pub use_this_parent: &'static str,
    pub clone_this_url: &'static str,
    pub already_exists: &'static str,
    pub clone_needs_update: &'static str,
    pub clone_search: &'static str,
    pub create_needs_update: &'static str,
    pub finder: &'static str,
    pub file_explorer: &'static str,
    pub file_manager: &'static str,
    pub search_failed: &'static str,
    pub not_found: &'static str,
    pub cannot_add: &'static str,
    pub cannot_clone: &'static str,
    pub cannot_create: &'static str,
    pub enter_name: &'static str,
    pub navigate: &'static str,
    pub select: &'static str,
    pub back: &'static str,
    pub close: &'static str,
    /// "Find a directory on {machine}".
    pub find_on: &'static str,
    /// "Choose or create a directory in {file manager}".
    pub browse_in: &'static str,
    /// "Create an empty directory on {machine}".
    pub create_on: &'static str,
    /// "Clone owner/repo via {HTTPS}".
    pub clone_via: &'static str,
    /// "{owner/repo} via {HTTPS}": the name, then the transport.
    pub repository_via: &'static str,
    /// "Parent directory: {parent}".
    pub parent_directory: &'static str,
}

/// English (Paseo's, host as machine).
pub const EN: AddFlowWords = AddFlowWords {
    choose_machine: "Choose machine",
    add_project: "Add project",
    search_for_directory: "Search for directory",
    browse: "Browse",
    clone_from_github: "Clone from GitHub",
    new_directory: "New directory",
    choose_destination: "Choose destination",
    choose_parent: "Choose parent directory",
    name_directory: "Name directory",
    search_machines: "Search machines...",
    search_directories: "Search directories or enter a path...",
    search_repository: "Search or enter a GitHub repository...",
    search_parents: "Search parent directories or enter a path...",
    directory_name: "Directory name",
    adding: "Adding project...",
    cloning: "Cloning project...",
    creating: "Creating directory...",
    loading: "Loading...",
    no_machines: "No connected machines",
    enter_repository: "Enter a GitHub URL or owner/repo",
    update_machine: "Update the machine to use Add Project.",
    no_matches: "No matching options",
    add_machine: "Add a machine",
    open_this_path: "Open this path",
    use_this_parent: "Use this parent",
    clone_this_url: "Clone this repository URL",
    already_exists: "Already exists",
    clone_needs_update: "Update this machine to clone GitHub repositories",
    clone_search: "Search projects available to your GitHub account",
    create_needs_update: "Update this machine to create directories",
    finder: "Finder",
    file_explorer: "File Explorer",
    file_manager: "the file manager",
    search_failed: "Unable to search directories",
    not_found: "Directory not found",
    cannot_add: "Unable to add project",
    cannot_clone: "Unable to clone repository",
    cannot_create: "Unable to create directory",
    enter_name: "Enter a directory name",
    navigate: "Navigate",
    select: "Select",
    back: "Back",
    close: "Close",
    find_on: "Find a directory on {}",
    browse_in: "Choose or create a directory in {}",
    create_on: "Create an empty directory on {}",
    clone_via: "Clone owner/repo via {}",
    repository_via: "{} via {}",
    parent_directory: "Parent directory: {}",
};

/// Korean.
pub const KO: AddFlowWords = AddFlowWords {
    choose_machine: "머신 선택",
    add_project: "프로젝트 추가",
    search_for_directory: "디렉터리 검색",
    browse: "찾아보기",
    clone_from_github: "GitHub에서 복제",
    new_directory: "새 디렉터리",
    choose_destination: "위치 선택",
    choose_parent: "상위 디렉터리 선택",
    name_directory: "디렉터리 이름",
    search_machines: "머신 검색...",
    search_directories: "디렉터리를 검색하거나 경로 입력...",
    search_repository: "GitHub 저장소를 검색하거나 입력...",
    search_parents: "상위 디렉터리를 검색하거나 경로 입력...",
    directory_name: "디렉터리 이름",
    adding: "프로젝트 추가 중...",
    cloning: "프로젝트 복제 중...",
    creating: "디렉터리 만드는 중...",
    loading: "불러오는 중...",
    no_machines: "연결된 머신 없음",
    enter_repository: "GitHub URL이나 owner/repo를 입력하세요",
    update_machine: "프로젝트 추가를 쓰려면 머신을 업데이트하세요.",
    no_matches: "맞는 항목 없음",
    add_machine: "머신 추가",
    open_this_path: "이 경로 열기",
    use_this_parent: "이 상위 디렉터리 사용",
    clone_this_url: "이 저장소 URL 복제",
    already_exists: "이미 있음",
    clone_needs_update: "GitHub 저장소를 복제하려면 이 머신을 업데이트하세요",
    clone_search: "GitHub 계정에서 쓸 수 있는 프로젝트 검색",
    create_needs_update: "디렉터리를 만들려면 이 머신을 업데이트하세요",
    finder: "Finder",
    file_explorer: "파일 탐색기",
    file_manager: "파일 관리자",
    search_failed: "디렉터리를 검색할 수 없음",
    not_found: "디렉터리를 찾을 수 없음",
    cannot_add: "프로젝트를 추가할 수 없음",
    cannot_clone: "저장소를 복제할 수 없음",
    cannot_create: "디렉터리를 만들 수 없음",
    enter_name: "디렉터리 이름을 입력하세요",
    navigate: "이동",
    select: "선택",
    back: "뒤로",
    close: "닫기",
    find_on: "{}에서 디렉터리 찾기",
    browse_in: "{}에서 디렉터리 고르거나 만들기",
    create_on: "{}에 빈 디렉터리 만들기",
    clone_via: "owner/repo를 {}로 복제",
    repository_via: "{} ({})",
    parent_directory: "상위 디렉터리: {}",
};

/// `template` with each `{}` taking the next of `values`.
fn fill(template: &str, values: &[&str]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut values = values.iter();
    let mut parts = template.split("{}");
    if let Some(first) = parts.next() {
        out.push_str(first);
    }
    for part in parts {
        out.push_str(values.next().copied().unwrap_or_default());
        out.push_str(part);
    }
    out
}

const fn transport(protocol: CloneProtocol) -> &'static str {
    match protocol {
        CloneProtocol::Https => "HTTPS",
        CloneProtocol::Ssh => "SSH",
    }
}

impl AddFlowWords {
    /// A method's name.
    #[must_use]
    pub const fn method(&self, method: Method) -> &'static str {
        match method {
            Method::DirectorySearch => self.search_for_directory,
            Method::Browse => self.browse,
            Method::Github => self.clone_from_github,
            Method::NewDirectory => self.new_directory,
        }
    }

    /// A row's first line.
    #[must_use]
    pub fn title(&self, title: &RowTitle) -> String {
        match title {
            RowTitle::Text(text) => text.clone(),
            RowTitle::AddMachine => self.add_machine.to_owned(),
            RowTitle::Method(method) => self.method(*method).to_owned(),
            RowTitle::RepositoryVia(name, protocol) => {
                fill(self.repository_via, &[name.as_str(), transport(*protocol)])
            }
        }
    }

    const fn file_manager_name(&self, manager: FileManager) -> &'static str {
        match manager {
            FileManager::Finder => self.finder,
            FileManager::FileExplorer => self.file_explorer,
            FileManager::Other => self.file_manager,
        }
    }

    /// A row's second line; Browse names `manager`.
    #[must_use]
    pub fn note(&self, note: &RowNote, manager: FileManager) -> String {
        match note {
            RowNote::Text(text) => text.clone(),
            RowNote::NoMachines => self.no_machines.to_owned(),
            RowNote::OpenThisPath => self.open_this_path.to_owned(),
            RowNote::UseThisParent => self.use_this_parent.to_owned(),
            RowNote::CloneThisUrl => self.clone_this_url.to_owned(),
            RowNote::CloneVia(protocol) => fill(self.clone_via, &[transport(*protocol)]),
            RowNote::AlreadyExists => self.already_exists.to_owned(),
            RowNote::ParentDirectory(parent) => fill(self.parent_directory, &[parent.as_str()]),
            RowNote::Method(method) => match method {
                MethodNote::FindOn(machine) => fill(self.find_on, &[machine.as_str()]),
                MethodNote::BrowseIn => fill(self.browse_in, &[self.file_manager_name(manager)]),
                MethodNote::CloneNeedsUpdate => self.clone_needs_update.to_owned(),
                MethodNote::CloneSearch => self.clone_search.to_owned(),
                MethodNote::CloneByHand => self.enter_repository.to_owned(),
                MethodNote::CreateOn(machine) => fill(self.create_on, &[machine.as_str()]),
                MethodNote::CreateNeedsUpdate => self.create_needs_update.to_owned(),
            },
        }
    }

    /// A page's title.
    #[must_use]
    pub const fn page_title(&self, page: PageId) -> &'static str {
        match page {
            PageId::Host => self.choose_machine,
            PageId::Method => self.add_project,
            PageId::DirectorySearch => self.search_for_directory,
            PageId::GithubSearch => self.clone_from_github,
            PageId::GithubLocation => self.choose_destination,
            PageId::NewDirectoryParent => self.choose_parent,
            PageId::NewDirectoryName => self.name_directory,
        }
    }

    /// The sheet's name for assistive technology: "Add project", then the
    /// page's title on a later page (the page's "Add project: <page>").
    #[must_use]
    pub fn sheet_label(&self, page: PageId) -> String {
        match page {
            PageId::Method => self.add_project.to_owned(),
            PageId::Host
            | PageId::DirectorySearch
            | PageId::GithubSearch
            | PageId::GithubLocation
            | PageId::NewDirectoryParent
            | PageId::NewDirectoryName => {
                format!("{}: {}", self.add_project, self.page_title(page))
            }
        }
    }

    /// A page's field placeholder; the method page has no field.
    #[must_use]
    pub const fn placeholder(&self, page: PageId) -> Option<&'static str> {
        match page {
            PageId::Host => Some(self.search_machines),
            PageId::Method => None,
            PageId::DirectorySearch => Some(self.search_directories),
            PageId::GithubSearch => Some(self.search_repository),
            PageId::GithubLocation | PageId::NewDirectoryParent => Some(self.search_parents),
            PageId::NewDirectoryName => Some(self.directory_name),
        }
    }

    /// The words while a page's call runs.
    #[must_use]
    pub const fn progress(&self, page: PageId) -> &'static str {
        match page {
            PageId::GithubLocation => self.cloning,
            PageId::NewDirectoryName => self.creating,
            PageId::Host
            | PageId::Method
            | PageId::DirectorySearch
            | PageId::GithubSearch
            | PageId::NewDirectoryParent => self.adding,
        }
    }

    /// The words when a page lists nothing.
    #[must_use]
    pub const fn empty(&self, empty: Empty) -> &'static str {
        match empty {
            Empty::NoMachines => self.no_machines,
            Empty::EnterRepository => self.enter_repository,
            Empty::UpdateMachine => self.update_machine,
            Empty::NoMatches => self.no_matches,
        }
    }
}

/// Add project's words of `lang`.
impl crate::text::Lang {
    /// Add project's words in this language.
    #[must_use]
    pub const fn add_flow(self) -> &'static AddFlowWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_go_where_each_language_reads_them() {
        let note = RowNote::Method(MethodNote::FindOn("laptop".to_owned()));
        assert_eq!(
            EN.note(&note, FileManager::Other),
            "Find a directory on laptop"
        );
        assert_eq!(
            KO.note(&note, FileManager::Other),
            "laptop에서 디렉터리 찾기"
        );
        let browse = RowNote::Method(MethodNote::BrowseIn);
        assert_eq!(
            EN.note(&browse, FileManager::Finder),
            "Choose or create a directory in Finder"
        );
        assert_eq!(
            EN.title(&RowTitle::RepositoryVia(
                "getpaseo/paseo".to_owned(),
                CloneProtocol::Ssh
            )),
            "getpaseo/paseo via SSH"
        );
        assert_eq!(fill("{} and {}", &["a"]), "a and ");
    }
}
