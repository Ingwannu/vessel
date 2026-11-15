//! File outcomes are a closed family, separate from unscoped workspace replies.
use super::answer::no;
use super::{FileAsk, PathChange};
use vessel_app_view::workspace::{
    file_ticket::FileTicket,
    problem::Problem,
    save::{SaveFailure, Saved},
};
use vessel_proto::workspace::{
    DirListing, FileRead, FindFiles, FoundFiles, GrepResult, ListDir, ReadFile, RelPath,
    SearchFiles, WorkspaceRoot,
};
/// Original file operation and its exact typed outcome.
#[derive(Debug)]
pub enum FileAnswer {
    /// A file-owned language query, correlated with its exact request.
    Intelligence {
        /// Original request, including source identity and ticket.
        ask: super::IntelligenceAsk,
        /// Typed language result or failure.
        result: super::IntelligenceResult,
    },
    /// A finite saved-file observation; never evidence a write ended.
    Observed {
        /// Original root/path/anchor.
        ask: vessel_proto::workspace::ObserveFile,
        /// Original acquisition identity.
        ticket: FileTicket,
        /// Current saved state or a typed failure.
        result: Result<vessel_proto::workspace::ObservedFile, Problem>,
    },
    /// Directory metadata, acquired together with its entries.
    Inspected {
        /// Original directory acquisition.
        ask: vessel_proto::workspace::InspectDirectory,
        /// Original refresh identity.
        ticket: FileTicket,
        /// One bounded snapshot or a typed failure.
        result: Result<vessel_proto::workspace::InspectedDirectory, Problem>,
    },
    /// A folder's entries.
    Listed {
        /// What was asked.
        ask: ListDir,
        /// Original local refresh or picker acquisition.
        ticket: FileTicket,
        /// The entries.
        result: Result<DirListing, Problem>,
    },
    /// A page of a file.
    Read {
        /// What was asked.
        ask: ReadFile,
        /// Original request identity, independent of the active pane.
        ticket: FileTicket,
        /// The page.
        result: Result<FileRead, Problem>,
    },
    /// A save's outcome.
    EditorRead {
        /// Requested source and byte budget.
        ask: ReadFile,
        /// The request identity; never inferred from current UI state.
        ticket: FileTicket,
        /// Exact read result.
        result: Result<FileRead, Problem>,
    },
    /// A save's outcome.
    Saved {
        /// The request identity.
        ticket: FileTicket,
        /// The root.
        root: WorkspaceRoot,
        /// The file.
        path: RelPath,
        /// What it came to.
        saved: Saved,
    },
    /// Files found by name.
    Found {
        /// What was asked.
        ask: FindFiles,
        /// The matches.
        result: Result<FoundFiles, Problem>,
    },
    /// A shared picker response, fenced independently of query text.
    TrackedFound {
        /// Original wire request.
        ask: FindFiles,
        /// Original acquisition generation.
        ticket: FileTicket,
        /// Matches or a typed refusal.
        result: Result<FoundFiles, Problem>,
    },
    /// Lines found in files.
    Grepped {
        /// What was asked.
        ask: SearchFiles,
        /// The matches.
        result: Result<GrepResult, Problem>,
    },
    /// A path made, renamed or deleted.
    PathChanged {
        /// Original app mutation identity.
        ticket: FileTicket,
        /// What was asked.
        change: PathChange,
        /// Done, or why not.
        result: Result<(), Problem>,
    },
}
impl FileAsk {
    /// A proven refusal before dispatch still settles the original owner ticket.
    #[must_use]
    pub fn refused(self, problem: &Problem) -> FileAnswer {
        let ask = self;
        match ask {
            FileAsk::Observe { ask, ticket } => FileAnswer::Observed {
                ask,
                ticket,
                result: no(problem),
            },
            FileAsk::Intelligence(ask) => FileAnswer::Intelligence {
                result: super::intelligence::refused(ask.action, problem),
                ask,
            },
            FileAsk::Inspect { ask, ticket } => FileAnswer::Inspected {
                ask,
                ticket,
                result: no(problem),
            },
            FileAsk::List { ask, ticket } => FileAnswer::Listed {
                ask,
                ticket,
                result: no(problem),
            },
            FileAsk::Read { ask, ticket } => FileAnswer::Read {
                ask,
                ticket,
                result: no(problem),
            },
            FileAsk::EditorRead { ask, ticket } => FileAnswer::EditorRead {
                ask,
                ticket,
                result: no(problem),
            },
            FileAsk::Save(save) => FileAnswer::Saved {
                ticket: save.ticket,
                root: save.root,
                path: save.path,
                saved: Saved::Failed(SaveFailure::Call(problem.clone())),
            },
            FileAsk::Find(ask) => FileAnswer::Found {
                ask,
                result: no(problem),
            },
            FileAsk::TrackedFind { ask, ticket } => FileAnswer::TrackedFound {
                ask,
                ticket,
                result: no(problem),
            },
            FileAsk::Grep(ask) => FileAnswer::Grepped {
                ask,
                result: no(problem),
            },
            FileAsk::Path { change, ticket } => FileAnswer::PathChanged {
                change,
                ticket,
                result: no(problem),
            },
        }
    }
}
