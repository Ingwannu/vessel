# ADR 0025: Files sent with a prompt

Status: accepted, 2026-11-11 (milestone 11, track A,
`docs/plans/backend-m11.md`, "Track A: contract" and its review; the
wire is protocol 10, `crates/vessel-proto/src/conversation/attachment.rs`).

## Context

All three references let a person attach images and files to a prompt.
ACP carries an image or audio in the prompt itself (`image`, `audio`
blocks, base64, when `promptCapabilities` says so) and any file as a
`resource_link` (baseline content); `embeddedContext` lets a client
embed a resource's text or bytes. The references move bytes three ways:
inline base64 in the prompt request (agent-orchestrator, 25 MiB of
images per request), a signed upload URL outside their RPC (GOAT), and a
chunked upload call (Paseo, 128 KiB pieces). They hand a non-image file
to the agent as a path: GOAT and Paseo from their state directory,
agent-orchestrator from a copy in the worktree.

Vessel's frames are at most 16 MiB, a call holds one of its connection's
slots while it runs, and every transport (websocket, relay, SSH, the
fleet's envelope) carries only RPC frames.

## Options

1. Inline bytes in `session.prompt`: one frame for every image of a
   prompt, larger than a frame for a large file, and one long frame holds
   up every other reply on the connection.
2. A separate HTTP upload with its own credential: the relay and SSH
   transports do not carry it, and it is a second authentication path.
3. Chunked calls over the connection, the reverse of protocol 6's
   `transfer.read`.

Where the bytes go:

1. Into the session's working folder (agent-orchestrator): agents with
   folder-bound tools read them, but the node writes into a person's
   project.
2. Under the node's data directory, per session (GOAT, Paseo).

## Decision

- `attachment.begin { session, name, mime, size }`, `attachment.write {
  upload, offset, data }` in pieces of at most 1 MiB, `attachment.finish
  { upload }`, which checks the size (and an image's first bytes against
  its declared type) and returns an `AttachmentRef`; `attachment.read`
  for another device to show it. Only the caller that began an upload
  writes to it; an upload idle 10 minutes, or open at a restart, is
  dropped. A repeated piece is answered with how much the node has.
- The kind follows the type: png, jpeg, gif and webp are images, `audio/*`
  is audio, anything else a file. Images and audio are offered only to an
  agent whose handshake takes them; files to every structured agent.
- The bytes are kept at `attachments/<session>/<id>/<name>` under the
  node's data directory, recorded by the store (size, type, SHA-256), and
  removed with the session (deleted or pruned); orphans are swept at
  start.
- To the agent: images and audio inline, a file as a `resource_link` to
  its stored path. The log keeps the reference only.
- A prompt names an attachment by id; the node uses its own record, not
  the client's. Read back (for the agent or for `attachment.read`) only
  as the regular file recorded at `finish`, never through a link.
- Limits a client reads in `Conversation.limits`: 10 MiB an image or
  audio file, 50 MiB another file, 16 a prompt with 25 MiB inline, 100 MiB
  a session (open uploads counted), 4 GiB a node, 16 uploads open at
  once and 4 per caller.

## Why

Chunked calls work on every transport Vessel has, through the fleet's
envelope too, and keep other calls flowing between pieces. Keeping the
bytes per session under the node's data directory leaves people's
projects untouched and gives them one owner that removes them. Reading a
file back only as the one recorded closes the path where an agent swaps
its attachment for a link to the node's token and a read-only device
fetches it.

## Consequences

- An agent whose tools stay inside its working folder may ask before it
  reads a file attachment (Claude asks a permission), or decline it.
- Attachments count against no event retention; their own bounds do.
- A session moved to another node does not take its attachments.

## As built (slice 6, 2026-11-11)

What the contract left open, decided while building it:

- **Who reads the bytes for the agent.** The node, never the session's
  actor. The runtime holds a sender to the node's loader
  (`vessel_session::AttachmentLoader`, a bounded channel of 16 requests;
  an actor finding no room waits); right before a session sends a prompt
  naming an attachment (a delivery, a queue's `send_now`, the queue
  moving), its actor asks the loader through `pump`, so the agent's
  output keeps being published while the node reads on its blocking
  pool. Loading before the prompt is handed over was rejected: a queued
  prompt would hold its bytes while it waits (25 MiB, 32 entries), and
  the queue's own sends would need the node anyway. The actor first
  checks what it can alone (16 attachments, 25 MiB inline, each named
  once, a kind its agent said it does not take), so nothing is read for
  a prompt refused anyway; the agent's driver checks the kind again
  against its own handshake, which covers a prompt sent while the agent
  starts.
- **Where the node's record replaces the client's.** Where a prompt
  enters the node: `session.prompt`, a queue `edit` and `session.send`.
  What waits in the queue, and what the store keeps of it, is therefore
  the node's record; the loader looks each one up again when it is sent.
- **Reading back.** The store owns the files' place and the read-back
  (it owns the data directory). On Unix every folder is opened relative
  to the one before with `O_NOFOLLOW | O_DIRECTORY` and the file with
  `O_NOFOLLOW`, through `rustix` (already in the tree for `vessel-fleet`);
  on Windows the path is checked with `symlink_metadata` before it is
  opened. Then the size and the SHA-256. A download's later pieces of an
  unchanged file are not hashed again: the caller passes the identity it
  verified (device, inode, size, modification and change times) and a
  file that still has it is read as is; replacing the file changes its
  inode, writing into it its change time. The node remembers 256 such
  identities. A file whose identity cannot be known (Windows) is hashed
  every time.
- **A file for the agent** is checked the same way (no hash once its
  identity was verified) and given as a `file:` URI, percent-encoded, to
  where it is kept; the node's data directory must be an absolute path.
- **A repeated piece.** A piece that ends at or before what the node has
  (sent again after a lost reply) is answered with how much it has and
  not written; one that starts anywhere else (a gap, or an overlap) is
  `invalid_request`.
- **An agent that has not said yet** (still starting) takes files only:
  `attachment.begin` of an image or audio is `unsupported` until its
  `ready`, so no image waits for a handshake that may refuse it.
- **Codes.** A kept file that was changed on disk is `conflict`, for
  `attachment.read` and for a prompt naming it; a row whose file is gone
  is `not_found` for `attachment.read` and `conflict` for a prompt. An
  image whose first bytes are not its type's is `invalid_request`, and
  its upload is dropped (it cannot become right). A prompt naming one
  attachment twice is `invalid_request`.
- **Staging.** An upload's bytes go to an owner-only file under the
  data directory's `uploads/`, hashed as they come, and are moved into
  place by `finish`; the staging directory is emptied at every start. A
  declared size counts against its session and the node from `begin`
  until its row is in the store.
- **Finishing for a session that ended.** The node checks the session is
  live, and the store records the row only for a live session in the
  same transaction (`StoreError::SessionEnded`); a refused row takes its
  file back.

## Native preview extension (proposed)

- Purpose and intent: display bounded historical images and open a fitted,
  zoomable preview from either a composer tray or a conversation message.
- Existing implementation and constraints: the node already owns and
  scopes attachment reads. The native view cache controls demand and the
  link fetches chunks without blocking the command loop.
- Alternatives considered: eagerly download a session, introduce another
  preview transport, or extend the existing attachment/read path.
- Selected approach: reuse the existing calls and GPUI decoder; keep
  fit/pan/zoom rules in the toolkit-free view crate. Link directly names
  the already linked `vessel-rpc::CallError` to preserve a failed read's
  source. This internal dependency adds no package or external license.
- Reason: eager loading grows with session history and a separate
  transport would duplicate the node's authorization. A typed RPC cause
  avoids parsing a string or erasing the diagnostic at the client edge.
- Advantages, disadvantages and effects: one bounded read pipeline and
  one overlay, with cache misses read again after eviction. The native
  dependency table records link's error-type edge. No protocol change.
