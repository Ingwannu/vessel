// The page's handles, branded (vessel-proto's `ts/aliases.rs`): the one
// module that turns a plain string or number into a `NodeKey`, a
// `SessionId` or another handle, each by a check of its shape.
//
// [Decision Log]
// - Purpose: the generated wire brands every handle minted by a node,
//   the bridge or this page's bridge client, so a session's id cannot be
//   passed where a machine's key is wanted. A frame narrowed by
//   `isServerFrame` already carries its brands; what is left is the few
//   places a handle enters the page as plain text: the address bar's
//   segments, the session's credential in the tab, the page's own
//   counters and nonces.
// - Choice: a type guard per handle here, at the bridge layer. Data and
//   routes may import it; features may not import `bridge/` at all
//   (eslint's layers), so no component can mint a brand. Tests brand
//   their fixtures' names with `data/fixtures/ids.ts`, the one helper
//   that does not check.
// - Each guard checks what the producer guarantees: a session's id is
//   the node's UUID (lowercase, as `uuid` writes it); a machine's key is
//   the book's name rule or `_here`; the bridge's launch and nonces are
//   64 lowercase hex digits; the client's counters are positive safe
//   integers. A segment that fails is a page that does not exist, said
//   before any call is sent.
// - Not checked here: values a person types (a book name, a title, a
//   path) are plain strings by the rule and are checked where they are
//   typed.

import type {
  CallId,
  LaunchId,
  NodeKey,
  Nonce,
  StreamId,
} from "../wire/bridge";
import type { ClientMessageId, SearchKey, SessionId } from "../wire/proto";

/** This machine's own node on the wire (`NodeKey::HERE`). */
const HERE_TEXT = "_here";

/** A book name: starts with a letter or digit, then letters, digits, `.`,
 * `_` or `-`, 64 bytes at most (vessel-client's `NodeName`). */
const BOOK_NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;

function unreachable(text: string): never {
  throw new Error(`Not a handle: ${text}`);
}

/** A UUID as the node writes one. */
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** 32 bytes in lowercase hex: the bridge's launch id and nonces. */
const HEX_32 = /^[0-9a-f]{64}$/;

/** Whether `text` names a machine as the bridge does: a book entry or
 * this machine's node. */
export function isNodeKey(text: string): text is NodeKey {
  return text === HERE_TEXT || BOOK_NAME.test(text);
}

/** Whether `text` is a session's id as a node mints it. */
export function isSessionId(text: string): text is SessionId {
  return UUID.test(text);
}

/** Mint once before preparing a submission; its owner retains it through uncertainty. */
export function newClientMessageId(): ClientMessageId {
  const value = crypto.randomUUID();
  return isClientMessageId(value)
    ? value
    : unreachable("a client message UUID");
}
function isClientMessageId(text: string): text is ClientMessageId {
  return UUID.test(text);
}

/** Whether `text` is a launch of `vessel ui`'s id. */
export function isLaunchId(text: string): text is LaunchId {
  return HEX_32.test(text);
}

function isNonce(text: string): text is Nonce {
  return HEX_32.test(text);
}

/** `text`, 32 random bytes in hex, as a nonce of the handshake's. */
export function nonceOf(text: string): Nonce {
  return isNonce(text) ? text : unreachable("a nonce");
}

function counted(value: number): boolean {
  return Number.isSafeInteger(value) && value > 0;
}

function isCallId(value: number): value is CallId {
  return counted(value);
}

function isStreamId(value: number): value is StreamId {
  return counted(value);
}

/** The client's counter `value` as a call's or an op's id. */
export function callIdOf(value: number): CallId {
  return isCallId(value) ? value : unreachable(String(value));
}

/** The client's counter `value` as a stream's id. */
export function streamIdOf(value: number): StreamId {
  return isStreamId(value) ? value : unreachable(String(value));
}

/** The largest search key the wire takes (the node's `u32`). */
export const SEARCH_KEY_MAX = 0xffff_ffff;

function isSearchKey(value: number): value is SearchKey {
  return counted(value) && value <= SEARCH_KEY_MAX;
}

/** The page's counter `value` as the key of one field's searches
 * (`file.find`): a newer search with the same key replaces an older one
 * still running. */
export function searchKeyOf(value: number): SearchKey {
  return isSearchKey(value) ? value : unreachable(String(value));
}

/** This machine's own node. */
export const HERE: NodeKey = isNodeKey(HERE_TEXT)
  ? HERE_TEXT
  : unreachable(HERE_TEXT);

/** `text` as a machine's key, or `null` when it is not one (an address
 * bar's segment). */
export function nodeKeyOf(text: string): NodeKey | null {
  return isNodeKey(text) ? text : null;
}

/** `text` as a session's id, or `null` when it is not one. */
export function sessionIdOf(text: string): SessionId | null {
  return isSessionId(text) ? text : null;
}
