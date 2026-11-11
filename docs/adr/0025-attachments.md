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
