// One file sent to a session in bounded pieces (milestone 11, Track
// A-page; ADR 0025): `attachment.begin` with its declared size, then
// `attachment.write` a piece at a time from what the node says it has,
// then `attachment.finish`, which answers the node's record of the file.
//
// [Decision Log]
// - Purpose: a 50 MB file must reach the node without the page holding
//   it whole, and a failed piece must stop that file, not the composer.
// - Constraint: the node takes at most `limits.chunk` bytes a piece,
//   base64 (one piece as text is a third larger again), and answers each
//   piece with what it has received, so a piece sent twice after a lost
//   answer is answered, not refused. Protocol 10 has no call to abandon an
//   upload: the node drops one idle for ten minutes, and an upload begun
//   holds its declared size against the session's room until then.
// - Alternatives: `FileReader.readAsDataURL` on the whole file (the file,
//   its base64 and the frame in memory at once); a stream reader (pieces
//   of the browser's size, not the node's).
// - Choice: `Blob.slice` and `arrayBuffer()` per piece, so at most one
//   piece of a file and its text are in memory while it is sent; the next
//   offset is the node's `received`, never the page's count. Stopping is
//   a signal checked between pieces. A retry writes the same upload on
//   from what the node has (beginning again would count the file twice
//   against the session's room); only an upload the node dropped
//   (`not_found`) is begun again.
// - Cost: one round trip a piece (a 1 MiB piece on a slow link is the
//   unit of progress), and an abandoned upload holds one of the caller's
//   four places on the node for ten minutes.

import type {
  AttachmentName,
  AttachmentRef,
  MimeType,
  Upload,
  UploadId,
  Written,
} from "../wire/proto";
import { errorCode, type Outcome, type Problem } from "./outcome";

/** The node's three upload calls, for one session. */
export interface UploadCalls {
  begin: (
    name: AttachmentName,
    mime: MimeType,
    size: number,
  ) => Promise<Outcome<Upload>>;
  write: (
    upload: UploadId,
    offset: number,
    data: string,
  ) => Promise<Outcome<Written>>;
  finish: (upload: UploadId) => Promise<Outcome<AttachmentRef>>;
}

/** An upload the node holds, and how far it got. */
export interface Begun {
  upload: UploadId;
  chunk: number;
  received: number;
}

/** How an upload ended. A failure keeps the upload when writing it again
 * may go on (`resume`), and says whether trying again can help at all. */
export type UploadEnd =
  | { kind: "done"; ref: AttachmentRef }
  | { kind: "failed"; problem: Problem; again: Again; resume: Begun | null }
  | { kind: "stopped"; resume: Begun | null };

/** Whether trying again can help: `never` when the node refused the file
 * itself (too large for what the session holds, a kind the agent does
 * not take, not the image it says it is, the session ended). */
export type Again = "yes" | "never";

/** What `send` reads of a file. */
export interface Source {
  size: number;
  slice: (start: number, end: number) => Blob;
}

/** `bytes` as base64, built a stretch at a time so a 1 MiB piece never
 * makes one argument list of a million numbers. */
export function base64Of(bytes: Uint8Array): string {
  const STRETCH = 0x8000;
  let binary = "";
  for (let start = 0; start < bytes.length; start += STRETCH) {
    binary += String.fromCharCode(...bytes.subarray(start, start + STRETCH));
  }
  return btoa(binary);
}

/** Whether a failure is worth trying again: the node's refusals of the
 * file are not. */
export function againAfter(problem: Problem): Again {
  switch (errorCode(problem)) {
    case "invalid_request":
    case "unsupported":
    case "conflict":
    case "forbidden":
      return "never";
    case null:
    case "unauthorized":
    case "protocol_mismatch":
    case "not_found":
    case "harness_not_installed":
    case "spawn_failed":
    case "unavailable":
    case "internal":
      return "yes";
  }
}

/** Send `source` as `name` of type `mime`, going on with `resume` when
 * the node still holds it (a new upload when it answers `not_found`),
 * reporting the bytes the node has after each piece, until done, failed,
 * or `stopped()` says so. */
export async function send(
  calls: UploadCalls,
  source: Source,
  name: AttachmentName,
  mime: MimeType,
  resume: Begun | null,
  progress: (received: number) => void,
  stopped: () => boolean,
): Promise<UploadEnd> {
  let held = resume;
  if (held === null) {
    const begun = await calls.begin(name, mime, source.size);
    if (!begun.ok) {
      return failed(begun.problem, null);
    }
    held = {
      upload: begun.value.upload,
      chunk: begun.value.chunk,
      received: 0,
    };
  }
  // A node that answered a piece size of zero would never finish: one
  // byte at a time is wrong but ends.
  const piece = Math.max(1, held.chunk);
  while (held.received < source.size) {
    if (stopped()) {
      return { kind: "stopped", resume: held };
    }
    const from = held.received;
    const end = Math.min(source.size, from + piece);
    const bytes = new Uint8Array(await source.slice(from, end).arrayBuffer());
    const written = await calls.write(held.upload, from, base64Of(bytes));
    if (!written.ok) {
      if (resume !== null && errorCode(written.problem) === "not_found") {
        // The node dropped the upload (idle, or restarted): begin again.
        return send(calls, source, name, mime, null, progress, stopped);
      }
      return failed(written.problem, held);
    }
    // The node's count, never ours: a piece it had already is answered
    // with what it has.
    if (written.value.received <= from) {
      return failed(
        {
          from: "op",
          reason: "failed",
          message: "The machine took none of the last piece",
        },
        held,
      );
    }
    held = { ...held, received: written.value.received };
    progress(held.received);
  }
  if (stopped()) {
    return { kind: "stopped", resume: held };
  }
  const finished = await calls.finish(held.upload);
  return finished.ok
    ? { kind: "done", ref: finished.value }
    : failed(finished.problem, null);
}

function failed(problem: Problem, resume: Begun | null): UploadEnd {
  const again = againAfter(problem);
  return {
    kind: "failed",
    problem,
    again,
    resume: again === "yes" ? resume : null,
  };
}
