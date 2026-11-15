//! Personal attention commands; answers reuse the session's forms.
use clap::Subcommand;
use vessel_proto::inbox::InboxCursor;

#[derive(Debug, Subcommand)]
pub(crate) enum InboxCommand {
    /// List the selected machine's inbox, newest first (including read entries).
    List,
    /// Show one entry and its currently available live form.
    Show {
        /// Inbox cursor printed by `inbox list`.
        entry: InboxCursor,
    },
    /// Answer one entry using its own approval option or question fields.
    Answer {
        /// Inbox cursor printed by `inbox list`.
        entry: InboxCursor,
        /// An approval's option id, or a question's field=value pairs.
        values: Vec<String>,
        /// Decline the request.
        #[arg(long, conflicts_with = "values")]
        decline: bool,
    },
    /// Mark an entry read, shared with the node's other paired devices.
    Read { entry: InboxCursor },
    /// Mark an entry unread without answering it.
    Unread { entry: InboxCursor },
}
