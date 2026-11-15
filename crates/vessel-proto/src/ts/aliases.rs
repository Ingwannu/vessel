//! What each primitive alias of the wire is in TypeScript: a branded
//! handle, or a plain string or number.
//!
//! [Decision Log]
//! - Purpose: in Rust a `SessionId` cannot be passed where a `NodeId` is
//!   expected; in the generated TypeScript both were `string`, so the
//!   page could swap them silently. A brand
//!   (`string & { readonly __brand: "SessionId" }`) gives the page the
//!   same check, with nothing changed on the wire: the brand exists only
//!   in the type.
//! - Constraint: a brand costs a check wherever a value enters the page
//!   from outside the types (a route parameter, a test's fixture). It is
//!   worth that where mixing two values is a real mistake and the value
//!   is never typed by a person.
//! - Rule: a handle minted by a node, the bridge, a harness or the page's
//!   bridge client is branded, whatever its representation (`SessionId`
//!   is a UUID string, `CallId` a counter). Everything else stays plain:
//!   - what a person types or names (a profile, a path, a title, a URL, a
//!     key binding): a form's text is one, and checking it is validation,
//!     not a brand;
//!   - quantities (sequence numbers, offsets, durations, sizes): they are
//!     counted, compared and added;
//!   - secrets: the page takes one from a person (a pasted link) and
//!     hands it on once; their rule is never to be kept or shown, which a
//!     brand does not say, and a guard would be one more place holding
//!     one;
//!   - content (terminal bytes, a draft's text): carried, not named.
//! - Numeric handles: branded. `CallId` and `StreamId` come from one
//!   counter in the page's bridge client and are looked up in different
//!   maps; the brand is what stops a reply's id from being looked up among
//!   the streams. `SubscriptionId` is the node's handle, branded by the
//!   same rule.
//! - Protocol 10's (milestone 11), by the same rule. Handles: what the
//!   node mints for an attachment, an upload, a question, a queued prompt,
//!   a project and a worktree; the two hashes the page only hands back
//!   (`ContentHash` with a file's write, `CommandsDigest` with an approval,
//!   as `BlobHash` is); a question's `FieldName`, which the agent names and
//!   the page echoes with the answer, as it does an `OptionId`; and
//!   `SearchKey`, the page's own number for a search, like `CallId`.
//!   Plain: what a person types or picks by name (a branch, a ref, a
//!   remote, a clone's URL and folder, a commit message, a project's name,
//!   a path in a project, a query, a `/` command, a file's name and type).
//!   `CommitId` is plain too: the node reports full ones, and a person may
//!   type an abbreviated one wherever a ref is taken. `ProjectCommand` is
//!   content: a line of a project's file, shown to be approved.
//! - Protocol 11's pull requests (track B2, ADR 0029), by the same rule,
//!   all plain: a forge's host, a repository's path, a login, a pull
//!   request's number, title and address, a check's name are what a person
//!   reads, types or picks (`#12`, `owner/name`); a pull request's
//!   description is content.
//! - Protocol 11's terminals (track C, ADR 0031), by the same rule. A
//!   handle: `TerminalId`, which the node mints. Plain: `ScriptName`, which
//!   a person types (`vessel script run dev`). Content: `ProcessName`, what
//!   the system called a listening process, shown and never handed back.
//! - Protocol 11's (track G, ADR 0032), by the same rule. Plain: an added
//!   agent's name, label and program, which a person chooses and types,
//!   an agent's version, an ACP registry entry's id and version, which a
//!   person reads and may type, and an archive's SHA-256, which a person
//!   compares. Handles: `InstallId`, which the node mints for an install,
//!   and `AuthMethodId`, which the agent names and the page only hands
//!   back, as an `OptionId`.
//! - Protocol 11's schedules and saved prompts (track D, ADR 0033), by the
//!   same rule. Handles: `ScheduleId` and `SavedPromptId`, which the node
//!   mints. Plain: a schedule's and a saved prompt's name, a cron
//!   expression and a time zone, which a person types (`--cron "0 9 * *
//!   1-5" --tz Europe/London`); quantities: `EverySeconds`, `MaxRuns` and
//!   a run's `RunNumber`, counted and compared. Content: `PromptText`, the
//!   text a schedule or a saved prompt sends, carried as a draft's is
//!   (track D had it plain as a name; the join moved it, plain either
//!   way).
//! - Project service declarations: `ServicePort` is a number a person chooses,
//!   like a pull-request number, not a node-minted handle or a count.
//!   `PortRange` and `PortScript` are also named choices a person writes.
//! - Kept current: every primitive alias the generator meets must be in
//!   `ALIASES` (`Declarations::unclassified` lists the others, and the
//!   generating crates' tests fail on one), and every entry must still be
//!   declared (the bridge crate's test, which sees both files, checks).
//!   A new alias is classified here, by the rule above, in the same
//!   change that adds it.
//! - Alternatives: branding every alias (a person's text would need a
//!   guard at every form field); branding none (the page's ids stay
//!   interchangeable); a `#[ts(brand)]` attribute on each type (ts-rs has
//!   none, and the rule would be spread over many files, with nothing to
//!   say an alias was never considered).
//! - Protocol 12's (track A2): `SearchCursor` is the node's handle for
//!   where a search goes on (a handle, though a number inside, as
//!   `CallId` is); `SearchQuery` is what a person types; `SearchLimit` a
//!   quantity.

/// What a primitive alias is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alias {
    /// A handle minted by a node, the bridge, a harness or the page's
    /// bridge client: branded.
    Handle,
    /// A name or text a person types or chooses (or the page's own
    /// action names): plain.
    Named,
    /// A count, a measure or a version: plain.
    Quantity,
    /// A secret: plain, and never kept or shown by the page.
    Secret,
    /// Bytes or text carried: plain.
    Content,
}

/// Every primitive alias of the protocol's and the bridge's declarations,
/// sorted by name.
pub const ALIASES: &[(&str, Alias)] = &[
    ("AcpProtocolVersion", Alias::Quantity),
    ("ActionId", Alias::Named),
    ("AgentLabel", Alias::Named),
    ("AgentMessageId", Alias::Handle),
    ("AgentName", Alias::Named),
    ("AgentProgram", Alias::Named),
    ("AgentVersion", Alias::Named),
    ("AttachmentId", Alias::Handle),
    ("AttachmentName", Alias::Named),
    ("AuthMethodId", Alias::Handle),
    ("BlobHash", Alias::Handle),
    ("BookName", Alias::Named),
    ("BranchName", Alias::Named),
    ("BundlePath", Alias::Named),
    ("CallId", Alias::Handle),
    ("CheckName", Alias::Named),
    ("CloneUrl", Alias::Named),
    ("CodeSize", Alias::Quantity),
    ("CommandName", Alias::Named),
    ("CommandsDigest", Alias::Handle),
    ("CommitId", Alias::Named),
    ("CommitMessage", Alias::Named),
    ("ConfigKey", Alias::Named),
    ("ConfigText", Alias::Content),
    ("ContentHash", Alias::Handle),
    ("CostUnit", Alias::Named),
    ("CronExpr", Alias::Named),
    ("CustomAgentSlot", Alias::Quantity),
    ("DeviceId", Alias::Handle),
    ("DiagnosticSchemaVersion", Alias::Quantity),
    ("DirectoryName", Alias::Named),
    ("DraftText", Alias::Content),
    ("DurationMs", Alias::Quantity),
    ("EnvName", Alias::Named),
    ("EventSeq", Alias::Quantity),
    ("EverySeconds", Alias::Quantity),
    ("FieldName", Alias::Handle),
    ("FilePath", Alias::Named),
    ("FindQuery", Alias::Named),
    ("FiniteNumber", Alias::Quantity),
    ("FolderName", Alias::Named),
    ("ForgeHost", Alias::Named),
    ("ForgeLogin", Alias::Named),
    ("ForgeUrl", Alias::Named),
    ("Generation", Alias::Quantity),
    ("GrepQuery", Alias::Named),
    ("HarnessSessionId", Alias::Handle),
    ("HeaderName", Alias::Named),
    ("HttpUrl", Alias::Named),
    ("IdleMinutes", Alias::Quantity),
    ("InboxCount", Alias::Quantity),
    ("InboxCursor", Alias::Quantity),
    ("InstallId", Alias::Handle),
    ("InterfaceSize", Alias::Quantity),
    ("InvitationSecret", Alias::Secret),
    ("KeyBinding", Alias::Named),
    ("LaunchId", Alias::Handle),
    ("LimitKey", Alias::Named),
    ("LimitName", Alias::Named),
    ("LimitPercent", Alias::Quantity),
    ("MaxRuns", Alias::Quantity),
    ("McpServerName", Alias::Named),
    ("MimeType", Alias::Named),
    ("ModelId", Alias::Named),
    ("ModelRouteName", Alias::Named),
    ("NodeId", Alias::Handle),
    ("NodeKey", Alias::Handle),
    ("NodePublicKey", Alias::Handle),
    ("Nonce", Alias::Handle),
    ("Offset", Alias::Quantity),
    ("OptionId", Alias::Handle),
    ("OptionValueId", Alias::Handle),
    ("PairingLink", Alias::Secret),
    ("PermissionId", Alias::Handle),
    ("PermissionOptionId", Alias::Handle),
    ("PortRange", Alias::Named),
    ("PortScript", Alias::Named),
    ("PresentedToken", Alias::Secret),
    ("PreviewCount", Alias::Quantity),
    ("ProcessName", Alias::Content),
    ("ProfileName", Alias::Named),
    ("ProjectCommand", Alias::Content),
    ("ProjectId", Alias::Handle),
    ("ProjectName", Alias::Named),
    ("PromptName", Alias::Named),
    ("PromptText", Alias::Content),
    ("ProtocolVersion", Alias::Quantity),
    ("ProviderName", Alias::Named),
    ("PullBody", Alias::Content),
    ("PullNumber", Alias::Named),
    ("PullTitle", Alias::Named),
    ("PushAuth", Alias::Secret),
    ("PushEndpoint", Alias::Secret),
    ("PushPublicKey", Alias::Content),
    ("QuestionId", Alias::Handle),
    ("QueueEntryId", Alias::Handle),
    ("RefName", Alias::Named),
    ("RegistryId", Alias::Named),
    ("RegistryRevision", Alias::Quantity),
    ("RegistryVersion", Alias::Named),
    ("RelPath", Alias::Named),
    ("RemoteName", Alias::Named),
    ("RepoPath", Alias::Named),
    ("ReportCount", Alias::Quantity),
    ("Revision", Alias::Quantity),
    ("RouteUrl", Alias::Named),
    ("RunNumber", Alias::Quantity),
    ("SavedPromptId", Alias::Handle),
    ("ScheduleId", Alias::Handle),
    ("ScheduleName", Alias::Named),
    ("ScriptName", Alias::Named),
    ("SearchCursor", Alias::Handle),
    ("SearchKey", Alias::Handle),
    ("SearchLimit", Alias::Quantity),
    ("SearchQuery", Alias::Named),
    ("SecretName", Alias::Named),
    ("SecretText", Alias::Secret),
    ("SecretValue", Alias::Secret),
    ("ServicePort", Alias::Named),
    ("SessionId", Alias::Handle),
    ("SessionTitle", Alias::Named),
    ("Sha256Hex", Alias::Named),
    ("StreamId", Alias::Handle),
    ("SubscriptionId", Alias::Handle),
    ("TerminalBytes", Alias::Content),
    ("TerminalId", Alias::Handle),
    ("TicketMac", Alias::Secret),
    ("TimeZoneName", Alias::Named),
    ("TokenCount", Alias::Quantity),
    ("ToolCallId", Alias::Handle),
    ("TransferId", Alias::Handle),
    ("UploadId", Alias::Handle),
    ("WorkspaceId", Alias::Handle),
    ("WorktreeId", Alias::Handle),
];

/// What the alias `name` is, if it is classified.
#[must_use]
pub fn classify(name: &str) -> Option<Alias> {
    ALIASES
        .binary_search_by(|(entry, _alias)| (*entry).cmp(name))
        .ok()
        .and_then(|at| ALIASES.get(at))
        .map(|(_name, alias)| *alias)
}

/// The primitive a declaration aliases, when `decl` (ts-rs's
/// `type Name = string;`) is one.
pub(super) fn primitive<'a>(decl: &'a str, name: &str) -> Option<&'a str> {
    let base = decl
        .strip_prefix("type ")?
        .strip_prefix(name)?
        .strip_prefix(" = ")?
        .strip_suffix(';')?;
    matches!(base, "string" | "number").then_some(base)
}

/// The declaration of the handle `name`, branded.
pub(super) fn branded(name: &str, base: &str) -> String {
    format!("type {name} = {base} & {{ readonly __brand: \"{name}\" }};")
}

#[cfg(test)]
mod tests {
    use super::super::Declarations;
    use super::*;

    /// An alias nobody has classified yet.
    #[derive(ts_rs::TS)]
    #[expect(dead_code, reason = "only its TypeScript declaration is used")]
    struct NotYetClassified(String);

    #[test]
    fn an_alias_not_in_the_table_is_reported_and_a_handle_is_branded() {
        let mut declarations = Declarations::new();
        declarations.add::<NotYetClassified>().unwrap();
        declarations.add::<crate::SessionId>().unwrap();
        declarations.add::<crate::EventSeq>().unwrap();
        assert_eq!(
            declarations.unclassified().collect::<Vec<_>>(),
            ["NotYetClassified"]
        );
        let text = declarations.render(None);
        assert!(
            text.contains(r#"export type SessionId = string & { readonly __brand: "SessionId" };"#),
            "{text}"
        );
        assert!(text.contains("export type EventSeq = number;"), "{text}");
    }

    #[test]
    fn the_table_is_sorted_and_each_name_is_classified_once() {
        for ((before, _), (after, _)) in ALIASES.iter().zip(ALIASES.iter().skip(1)) {
            assert!(before < after, "{before} before {after}");
        }
        assert_eq!(classify("SessionId"), Some(Alias::Handle));
        assert_eq!(classify("SessionTitle"), Some(Alias::Named));
        assert_eq!(classify("PairingLink"), Some(Alias::Secret));
        assert_eq!(classify("EventSeq"), Some(Alias::Quantity));
        assert_eq!(classify("Nothing"), None);
    }

    #[test]
    fn only_string_and_number_aliases_are_primitive() {
        assert_eq!(
            primitive("type SessionId = string;", "SessionId"),
            Some("string")
        );
        assert_eq!(primitive("type CallId = number;", "CallId"), Some("number"));
        assert_eq!(primitive("type A = string | null;", "A"), None);
        assert_eq!(primitive("type A = { a: string };", "A"), None);
        assert_eq!(primitive("type AB = string;", "A"), None);
        assert_eq!(
            branded("SessionId", "string"),
            r#"type SessionId = string & { readonly __brand: "SessionId" };"#
        );
    }
}
