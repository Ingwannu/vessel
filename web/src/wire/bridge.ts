// Generated from the Rust wire types (ADR 0017). Do not edit: running
// `VESSEL_WRITE_TS=1 cargo test --features ts typescript` in the crate that
// generates this file rewrites it, and CI fails while it differs.

import type { Call, EventRecord, EventSeq, HarnessKind, Method, NodeId, NodeInfo, OptionId, OptionValueId, PairingOffer, ProfileName, ProtocolVersion, Reply, SessionId, TerminalId, WireError } from "./proto";

/**
 * One of the page's actions: 1 to 48 characters of `a-z`, `0-9` and `-`,
 * starting with a letter. The page's own ids; the bridge does not know
 * the set.
 */
export type ActionId = string;

/**
 * What one agent starts with on one machine.
 */
export type AgentDefaults = { 
/**
 * The profile; `None` is the harness's default.
 */
profile: ProfileName | null, 
/**
 * The agent's own options and their values, applied only while the
 * agent still offers them. At most [`MAX_CHOICES`].
 */
choices: { [key in ChoiceKey]: ChoiceValue }, };

/**
 * How a notice is announced.
 */
export type Alerts = "off" | "system" | "sound" | "system_and_sound";

/**
 * How the page looks.
 */
export type Appearance = { 
/**
 * Light, dark, or the system's.
 */
theme: Theme, 
/**
 * The interface's text size.
 */
interface_size: InterfaceSize, 
/**
 * Code's text size.
 */
code_size: CodeSize, 
/**
 * How wide the conversation is.
 */
content_width: ContentWidth, 
/**
 * How a conversation shows tool calls.
 */
conversation: ConversationDisplay, };

/**
 * Which key proves the connection: never send the key itself in a hello.
 */
export type AuthMode = "bootstrap" | "resume";

/**
 * A node's name in the book, where only a book entry will do.
 */
export type BookName = string;

/**
 * A node the UI lists.
 */
export type BookNode = { 
/**
 * How the browser names it.
 */
key: NodeKey, 
/**
 * The book's name for it, or a description of this machine's node.
 */
label: string, 
/**
 * Where it is, as the book has it (an address, an SSH target, a
 * paired node's key fingerprint).
 */
address: string, 
/**
 * How it is reached.
 */
transport: Transport, 
/**
 * Whether commands use it when none is named.
 */
default: boolean, 
/**
 * Its id, once seen.
 */
id: NodeId | null, };

/**
 * The browser's id for a call or an op.
 */
export type CallId = number & { readonly __brand: "CallId" };

/**
 * A call's answer.
 */
export type CallOutcome = { "ok": Reply } | { "err": Failure };

/**
 * An agent's option id remembered with its value: an [`OptionId`] of at
 * most [`MAX_CHOICE_TEXT`] bytes.
 */
export type ChoiceKey = OptionId;

/**
 * The value a client asks an option to take.
 */
export type ChoiceValue = { "kind": "select", 
/**
 * The value.
 */
value: OptionValueId, } | { "kind": "toggle", 
/**
 * Whether to turn it on.
 */
on: boolean, };

/**
 * A frame from the browser.
 */
export type ClientFrame = { "type": "hello", 
/**
 * Bootstrap or a session resumed by this tab.
 */
mode: AuthMode, 
/**
 * The browser's fresh challenge; binds the welcome to this socket.
 */
nonce: Nonce, 
/**
 * HMAC of both challenges, launch id and role (never the key).
 */
proof: PresentedToken, } | { "type": "call", 
/**
 * The browser's id for the call, echoed in the reply.
 */
id: CallId, 
/**
 * The node.
 */
node: NodeKey, 
/**
 * The protocol's own `Call` (`{ "method", "params" }`). Kept as
 * text until the frame is read, then decoded into the typed
 * `Call`, so that a call this bridge cannot read is still
 * answered, by id.
 */
call: Call, } | { "type": "follow", 
/**
 * The browser's id for the stream.
 */
stream: StreamId, 
/**
 * The node.
 */
node: NodeKey, 
/**
 * The session.
 */
session: SessionId, 
/**
 * The first record wanted.
 */
from: EventSeq, } | { "type": "follow_terminal", 
/**
 * The browser's id for the stream.
 */
stream: StreamId, 
/**
 * The node.
 */
node: NodeKey, 
/**
 * The terminal.
 */
terminal: TerminalId, 
/**
 * The first record wanted.
 */
from: EventSeq, } | { "type": "unfollow", 
/**
 * The stream.
 */
stream: StreamId, } | { "type": "op", 
/**
 * The browser's id for the request, echoed in the answer.
 */
id: CallId, 
/**
 * What to do.
 */
op: Op, };

/**
 * Code's text size: whole pixels, 10 to 18 (12 by default).
 */
export type CodeSize = number;

/**
 * The composer.
 */
export type Composer = { 
/**
 * Which key sends.
 */
send: SendKey, 
/**
 * What Send does while a turn runs.
 */
delivery: TurnDelivery, };

/**
 * How wide the conversation is.
 */
export type ContentWidth = "comfortable" | "wide" | "full";

/**
 * How a conversation shows tool calls.
 *
 * Its name says display because the bare word is taken: the protocol's
 * type of that name is a session's queue, commands and questions
 * (protocol 10), and the page's two generated files share one namespace
 * of type names. The field (`conversation`) and the change's tag keep
 * their names, so a preferences file written before the rename reads as
 * it did. (The bare name is not written here either: the generator
 * imports from the protocol's file every protocol type a declaration
 * mentions, its comment included.)
 */
export type ConversationDisplay = "full" | "collapse_tools";

/**
 * One draft.
 */
export type Draft = { 
/**
 * Whose composer.
 */
key: DraftKey, 
/**
 * The text (never empty: an empty save removes the draft).
 */
text: DraftText, 
/**
 * When it was last saved.
 */
at: string, };

/**
 * Whose composer a draft is: the new-session page's (there is one), or a
 * session's. A session id is minted by its node and unique across
 * machines, so a renamed machine keeps its drafts.
 */
export type DraftKey = { "kind": "new" } | { "kind": "session", 
/**
 * The session.
 */
session: SessionId, };

/**
 * A draft's text: at most [`MAX_DRAFT`] bytes of UTF-8; the empty text
 * removes the draft. `Debug` shows its length only: a draft is whatever
 * a person typed, and has no place in a log line or a test's failure.
 */
export type DraftText = string;

/**
 * Every draft, and where they are kept.
 */
export type DraftsState = { 
/**
 * Where they are kept.
 */
kept: Kept, 
/**
 * The drafts, ordered by key (the new-session page's first).
 */
drafts: Array<Draft>, };

/**
 * Why a call has no reply.
 */
export type Failure = { "kind": "remote", 
/**
 * The node's error.
 */
error: WireError, } | { "kind": "not_in_protocol", 
/**
 * The method.
 */
method: Method, 
/**
 * The version it needs.
 */
needs: ProtocolVersion, 
/**
 * The node's.
 */
negotiated: ProtocolVersion, } | { "kind": "disconnected" } | { "kind": "unreachable", 
/**
 * Why.
 */
message: string, } | { "kind": "refused", 
/**
 * Why.
 */
message: string, } | { "kind": "failed", 
/**
 * What happened.
 */
message: string, };

/**
 * What the sidebar groups sessions by.
 */
export type Grouping = "machine" | "status" | "workspace";

/**
 * The interface's text size: whole pixels, 12 to 20 (14 by default).
 */
export type InterfaceSize = number;

/**
 * Where the preferences or the drafts are kept.
 */
export type Kept = { "kind": "file", 
/**
 * The file, as display text.
 */
path: string, } | { "kind": "this_launch", 
/**
 * Why not in a file.
 */
why: LaunchOnly, };

/**
 * The keys that run an action: one chord, or two pressed in sequence
 * (`G S`), separated by one space, at most 32 bytes.
 *
 * A chord is its modifiers then its key, joined by `+`. The modifiers are
 * `Mod` (Command on a Mac, Control elsewhere), `Ctrl`, `Alt`, `Shift` and
 * `Meta`, each at most once and in that order. A key names a physical key
 * (`KeyboardEvent.code`) as it is written on a US keyboard without Shift:
 * `A` to `Z`, `0` to `9`, `` ` - = [ ] \ ; ' , . / ``, or one of `Enter`,
 * `Escape`, `Tab`, `Space`, `Backspace`, `Delete`, `Up`, `Down`, `Left`,
 * `Right`, `Home`, `End`, `PageUp`, `PageDown`, `F1` to `F12`. So `?` is
 * `Shift+/`, and each key press has exactly one spelling.
 */
export type KeyBinding = string;

/**
 *The public identity of one UI process launch.
 */
export type LaunchId = string & { readonly __brand: "LaunchId" };

/**
 * Why something is kept for this launch only, for the page to say
 * without reading words.
 */
export type LaunchOnly = { "kind": "no_place" } | { "kind": "no_path", 
/**
 * Why.
 */
message: string, } | { "kind": "unreadable", 
/**
 * The file, as display text.
 */
path: string, 
/**
 * Why.
 */
message: string, } | { "kind": "newer", 
/**
 * The file, as display text.
 */
path: string, 
/**
 * Its format's version.
 */
version: number, };

/**
 * Which panels show.
 */
export type Layout = { 
/**
 * The sidebar.
 */
sidebar: Panel, 
/**
 * The inspector (a session's summary).
 */
inspector: Panel, };

/**
 * What a new session on one machine starts with, remembered from the
 * last one started there.
 */
export type MachineDefaults = { 
/**
 * The agent picked first.
 */
agent: HarnessKind | null, 
/**
 * What each agent starts with.
 */
agents: { [key in HarnessKind]?: AgentDefaults }, };

/**
 * How the browser names a node: a book entry, or this machine's node
 * ([`NodeKey::HERE`] on the wire). Book names start with a letter or a
 * digit, so the two never meet.
 */
export type NodeKey = string & { readonly __brand: "NodeKey" };

/**
 * Where a node's connection is.
 */
export type NodeStatus = { "state": "idle" } | { "state": "connecting" } | { "state": "connected", 
/**
 * The node, as it introduced itself.
 */
node: NodeInfo, 
/**
 * The protocol version in use.
 */
protocol: ProtocolVersion, } | { "state": "reconnecting", 
/**
 * Why it is down.
 */
message: string, 
/**
 * Which attempt comes next, from 1.
 */
attempt: number, 
/**
 * Milliseconds until it.
 */
retry_in_ms: number, } | { "state": "failed", 
/**
 * Why.
 */
message: string, };

/**
 *A fresh 256-bit connection challenge.
 */
export type Nonce = string & { readonly __brand: "Nonce" };

/**
 * How the page tells about sessions.
 */
export type Notifications = { 
/**
 * How a session that needs the person, or finished, is announced.
 */
alerts: Alerts, 
/**
 * A count of notices in the tab's title.
 */
title: boolean, };

/**
 * What the browser asks the bridge itself.
 */
export type Op = { "op": "nodes" } | { "op": "retry", 
/**
 * The node.
 */
node: NodeKey, } | { "op": "inspect", 
/**
 * The link (`vessel://pair#...`).
 */
link: SecretText, } | { "op": "pair", 
/**
 * The name to give it in the book.
 */
name: BookName, 
/**
 * The link.
 */
link: SecretText, } | { "op": "qr", 
/**
 * What to encode (a pairing link).
 */
text: SecretText, } | { "op": "forget", 
/**
 * The node.
 */
node: BookName, } | { "op": "rename", 
/**
 * The node.
 */
node: BookName, 
/**
 * Its new name. The same name changes nothing.
 */
name: BookName, } | { "op": "default", 
/**
 * The node.
 */
node: NodeKey, } | { "op": "preferences" } | { "op": "prefer", 
/**
 * The change.
 */
change: PreferenceChange, } | { "op": "drafts" } | { "op": "draft", 
/**
 * Whose.
 */
key: DraftKey, } | { "op": "save_draft", 
/**
 * Whose.
 */
key: DraftKey, 
/**
 * The text.
 */
text: DraftText, };

/**
 * Why an op has no result.
 */
export type OpFailure = "unreadable" | "busy" | "failed";

/**
 * An op's answer.
 */
export type OpOutcome = { "ok": OpResult } | { "err": { 
/**
 * Which kind of failure, for the page to act on without reading
 * `message`.
 */
reason: OpFailure, 
/**
 * Why, in words.
 */
message: string, } };

/**
 * An op's result.
 */
export type OpResult = { "op": "nodes", 
/**
 * The nodes, in the order to show them.
 */
nodes: Array<BookNode>, } | { "op": "retry" } | { "op": "inspect", 
/**
 * The offer.
 */
offer: PairingOffer, 
/**
 * The fingerprint of its key, to compare with what the node
 * shows.
 */
fingerprint: string, } | { "op": "pair", 
/**
 * The node as the book has it now.
 */
node: BookNode, } | { "op": "qr", 
/**
 * The code as an SVG document.
 */
svg: string, } | { "op": "forget", 
/**
 * The nodes as the book has them now, in the order to show them.
 */
nodes: Array<BookNode>, } | { "op": "rename", 
/**
 * The nodes as the book has them now, in the order to show them.
 */
nodes: Array<BookNode>, } | { "op": "default", 
/**
 * The nodes as the book has them now, in the order to show them
 * (the default moved: two entries changed).
 */
nodes: Array<BookNode>, } | { "op": "preferences", 
/**
 * The state now.
 */
preferences: PreferencesState, } | { "op": "prefer", 
/**
 * The state after the change (also pushed to every tab).
 */
preferences: PreferencesState, } | { "op": "drafts", 
/**
 * Every draft.
 */
drafts: DraftsState, } | { "op": "draft", 
/**
 * Its text; `None` when there is no such draft.
 */
text: DraftText | null, } | { "op": "save_draft" };

/**
 * Whether a panel shows.
 */
export type Panel = "shown" | "hidden";

/**
 * One change to the preferences: each replaces one setting.
 */
export type PreferenceChange = { "set": "theme", "value": Theme } | { "set": "interface_size", "value": InterfaceSize } | { "set": "code_size", "value": CodeSize } | { "set": "content_width", "value": ContentWidth } | { "set": "conversation", "value": ConversationDisplay } | { "set": "sidebar_panel", "value": Panel } | { "set": "inspector_panel", "value": Panel } | { "set": "send", "value": SendKey } | { "set": "delivery", "value": TurnDelivery } | { "set": "alerts", "value": Alerts } | { "set": "title_count", "value": boolean } | { "set": "sidebar_group", "value": Grouping } | { "set": "sidebar_sort", "value": SessionSort } | { "set": "sidebar_ended", "value": boolean } | { "set": "sidebar_labels", "value": { 
/**
 * The names picked; `""` is Unlabelled.
 */
picks: Array<string>, } } | { "set": "shortcuts", "value": { 
/**
 * At most [`MAX_EDITS`].
 */
edits: Array<ShortcutEdit>, } } | { "set": "reset_shortcuts" } | { "set": "default_agent", "value": { 
/**
 * The machine.
 */
node: NodeId, 
/**
 * The agent; `None` forgets it.
 */
agent: HarnessKind | null, } } | { "set": "agent_defaults", "value": { 
/**
 * The machine.
 */
node: NodeId, 
/**
 * The agent.
 */
agent: HarnessKind, 
/**
 * The profile; `None` is the harness's default.
 */
profile: ProfileName | null, 
/**
 * The agent's options, at most [`MAX_CHOICES`]; an id or a value
 * longer than [`MAX_CHOICE_TEXT`] bytes is not remembered.
 */
choices: { [key in ChoiceKey]: ChoiceValue }, } } | { "set": "forget_defaults", "value": { 
/**
 * The machine.
 */
node: NodeId, } };

/**
 * Every preference. A section or a field the file does not have is its
 * default.
 */
export type Preferences = { 
/**
 * How the page looks.
 */
appearance: Appearance, 
/**
 * Which panels show.
 */
layout: Layout, 
/**
 * The composer.
 */
composer: Composer, 
/**
 * Keys the person chose, by action: an action not here has its
 * default keys; one with an empty list has none. At most
 * [`MAX_ACTIONS`].
 */
shortcuts: { [key in ActionId]: Array<KeyBinding> }, 
/**
 * How the page tells about sessions.
 */
notifications: Notifications, 
/**
 * How the sidebar lists sessions.
 */
sidebar: SidebarView, 
/**
 * What new sessions start with, by machine. At most [`MAX_MACHINES`].
 */
defaults: { [key in NodeId]: MachineDefaults }, };

/**
 * Where the preferences are now, with every value.
 */
export type PreferencesState = { 
/**
 * Rises by one with every state the bridge publishes in this launch;
 * the page drops a reply or a push older than what it shows.
 */
revision: Revision, 
/**
 * Where they are kept.
 */
kept: Kept, 
/**
 * The values.
 */
values: Preferences, };

/**
 * A token as a browser presented it: not yet checked, never printed.
 */
export type PresentedToken = string;

/**
 * The number of a published preferences state within one launch, from
 * 1.
 */
export type Revision = number;

/**
 * Text from the page that may be a credential: a pairing link (to
 * inspect or pair with), or the text of a QR code (a pairing link to
 * show). A link holds a single-use invitation until it is redeemed, so
 * `Debug` never prints it (an `Op` or a `ClientFrame` in a log line, a
 * test failure or a panic message shows a placeholder), and there is no
 * `Display`. It is not checked here: explaining a bad link is
 * `inspect`'s job, with the book's own parser.
 *
 * [`SecretText::expose`] is called only where the text is parsed or
 * drawn (`ops`: inspect, pair, qr; the source's pairing); nothing on
 * those paths logs it.
 */
export type SecretText = string;

/**
 * Which key sends a prompt.
 */
export type SendKey = "enter" | "mod_enter";

/**
 * A frame to the browser.
 */
export type ServerFrame = { "type": "challenge", 
/**
 * This UI launch.
 */
launch: LaunchId, 
/**
 * A fresh random value, used on this connection only.
 */
nonce: Nonce, } | { "type": "welcome", 
/**
 * The server's version (`vessel`'s).
 */
version: string, 
/**
 * Must match the challenge and the tab's previous launch.
 */
launch: LaunchId, 
/**
 * HMAC including the client nonce and any issued session key.
 */
proof: PresentedToken, 
/**
 * An independent key, only when redeeming the one-time code.
 */
session: PresentedToken | null, } | { "type": "reply", 
/**
 * The call's id.
 */
id: CallId, 
/**
 * The node's reply, or why there is none.
 */
outcome: CallOutcome, } | { "type": "op_reply", 
/**
 * The op's id.
 */
id: CallId, 
/**
 * Its result, or why there is none.
 */
outcome: OpOutcome, } | { "type": "node", 
/**
 * The node.
 */
node: NodeKey, 
/**
 * Where its connection is now.
 */
status: NodeStatus, } | { "type": "event", 
/**
 * The stream.
 */
stream: StreamId, 
/**
 * The record (gap-free and in order, within one stream).
 */
record: EventRecord, } | { "type": "stream", 
/**
 * The stream.
 */
stream: StreamId, 
/**
 * What happened.
 */
state: StreamState, } | { "type": "unreadable", 
/**
 * Why.
 */
message: string, } | { "type": "preferences", 
/**
 * The state, with its revision.
 */
preferences: PreferencesState, };

/**
 * The order of sessions within a group.
 */
export type SessionSort = "created" | "activity";

/**
 * One action's new keys.
 */
export type ShortcutEdit = { 
/**
 * The action.
 */
action: ActionId, 
/**
 * Its keys from now on.
 */
to: ShortcutTarget, };

/**
 * What an action's keys become.
 */
export type ShortcutTarget = { "kind": "bindings", 
/**
 * At most [`MAX_BINDINGS`].
 */
keys: Array<KeyBinding>, } | { "kind": "default" };

/**
 * How the sidebar lists sessions.
 */
export type SidebarView = { 
/**
 * What sessions are grouped by.
 */
group: Grouping, 
/**
 * Their order within a group.
 */
sort: SessionSort, 
/**
 * Whether ended sessions are listed.
 */
ended: boolean, 
/**
 * The workspace labels the sidebar is filtered by, when grouped by
 * workspace; empty lists every workspace, `""` is Unlabelled. At most
 * 32 names.
 */
labels: Array<string>, };

/**
 * `GET /api/status`: what is serving.
 */
export type Status = { 
/**
 * The server's version (`vessel`'s).
 */
version: string, 
/**
 * Whether the real UI is embedded (else the placeholder page).
 */
bundled: boolean, };

/**
 * The browser's id for a followed session.
 */
export type StreamId = number & { readonly __brand: "StreamId" };

/**
 * What happened to a followed session's stream.
 */
export type StreamState = { "state": "missing", 
/**
 * The first record that was wanted.
 */
expected: EventSeq, 
/**
 * Where it continues.
 */
resumed_at: EventSeq, } | { "state": "reconnecting", 
/**
 * Why.
 */
message: string, } | { "state": "resumed", 
/**
 * The next record.
 */
at: EventSeq, } | { "state": "ended" } | { "state": "failed", 
/**
 * Why.
 */
message: string, 
/**
 * Which kind of stop, for the page to act on without reading
 * `message`.
 */
reason: StreamStop, };

/**
 * Why a followed session's stream stopped for good.
 */
export type StreamStop = "removed" | "stopping" | "error";

/**
 * The page's colours.
 */
export type Theme = "system" | "light" | "dark";

/**
 * How a node is reached.
 */
export type Transport = "ws" | "ssh" | "paired";

/**
 * What Send does with a prompt while the agent's turn runs: protocol
 * 10's ways of delivering one, less `start` (what Send does between
 * turns).
 *
 * [Decision Log]
 * - Purpose: agent-orchestrator, GOAT and Paseo each let a person choose
 *   whether a message typed during a turn waits for it, joins it or
 *   stops it; GOAT and T3 Code keep the default as a setting.
 * - Choice: the default kept here, with the bridge's other preferences
 *   (ADR 0028), so every tab and every launch sends the same way. Its
 *   name is not the protocol's for the same set: the page's two
 *   generated files share one namespace of type names, and the
 *   protocol's has `start` too. (Nor is that name written here: the
 *   generator imports every protocol type a comment mentions.)
 * - Steer stays a choice even for an agent that cannot steer: the page
 *   then queues and says so, and the person's choice holds for the next
 *   agent that can.
 */
export type TurnDelivery = "queue" | "steer" | "interrupt";
