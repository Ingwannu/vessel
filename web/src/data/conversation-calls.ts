// The calls of protocol 10's conversation the page makes (milestone 11,
// Track A-page): sending with a delivery, the node's queue, the
// conversation's state, answering a question, a session's marks, starting
// again, deleting, and attachments. Each answers an `Outcome` (a typed
// value or a typed failure, nothing thrown), as `calls.ts` does for the
// calls before it; they live apart so that file stays one subject.

import type { NodeKey } from "../wire/bridge";
import type {
  AttachmentId,
  AttachmentName,
  AttachmentRef,
  BlobChunk,
  ContentBlock,
  Conversation,
  Delivered,
  Delivery,
  EventSeq,
  MimeType,
  QuestionAnswer,
  QuestionId,
  QueueChanged,
  QueueEntryId,
  SessionDeleted,
  SessionId,
  SessionInfo,
  Upload,
  UploadId,
  Written,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";

/** One change to a session's queue, as `session.queue` takes it. */
export type QueueEdit =
  | {
      change: "edit";
      entry: QueueEntryId;
      revision: number;
      content: ContentBlock[];
    }
  | { change: "move"; entry: QueueEntryId; before?: QueueEntryId | null }
  | { change: "remove"; entry: QueueEntryId }
  | {
      change: "send_now";
      entry: QueueEntryId;
      delivery: Exclude<Delivery, "queue">;
    }
  | { change: "hold" }
  | { change: "resume" }
  | { change: "clear" };

/** A mark, as `session.mark` takes it. */
export type MarkChange =
  | { mark: "pin" }
  | { mark: "unpin" }
  | { mark: "archive" }
  | { mark: "unarchive" }
  | { mark: "read"; through: EventSeq }
  | { mark: "unread" };

/** Send `content` to `session`, delivered as `delivery`. */
export function promptSession(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  content: ContentBlock[],
  delivery: Delivery,
): Promise<Outcome<Delivered>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.prompt", { session, content, delivery }),
  );
}

/** Change `session`'s queue. The answer is the queue after it. */
export function changeQueue(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  edit: QueueEdit,
): Promise<Outcome<QueueChanged>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.queue", { session, ...edit }),
  );
}

/** The conversation as it stands: what the page reads of it is the
 * attachment limits (the log carries the rest). */
export function readConversation(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
): Promise<Outcome<Conversation>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.conversation", { session }),
  );
}

/** Answer the agent's question `question`. The first answer wins. */
export function answerQuestion(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  question: QuestionId,
  answer: QuestionAnswer,
): Promise<Outcome<null>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.send", {
      session,
      input: { type: "answer_question", id: question, answer },
    }),
  );
}

/** Mark `session` (pin, archive, read…). The answer is the session with
 * its marks after. */
export function markSession(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  mark: MarkChange,
): Promise<Outcome<SessionInfo>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.mark", { session, ...mark }),
  );
}

/** Start an ended session's agent again with its conversation. Answers
 * once the agent is running, or why it did not start. */
export function resumeSession(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
): Promise<Outcome<SessionInfo>> {
  return attempt(() => fleet.bridge.call(node, "session.resume", { session }));
}

/** Delete an ended session for good: its log, marks, queue and
 * attachments. The answer says what became of its worktree. */
export function deleteSession(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
): Promise<Outcome<SessionDeleted>> {
  return attempt(() => fleet.bridge.call(node, "session.delete", { session }));
}

/** Begin uploading a file of `size` bytes to `session`. */
export function beginUpload(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  name: AttachmentName,
  mime: MimeType,
  size: number,
): Promise<Outcome<Upload>> {
  return attempt(() =>
    fleet.bridge.call(node, "attachment.begin", { session, name, mime, size }),
  );
}

/** One piece of an upload, base64, starting at `offset`. */
export function writeUpload(
  fleet: Fleet,
  node: NodeKey,
  upload: UploadId,
  offset: number,
  data: string,
): Promise<Outcome<Written>> {
  return attempt(() =>
    fleet.bridge.call(node, "attachment.write", { upload, offset, data }),
  );
}

/** Finish an upload whose every byte was written. */
export function finishUpload(
  fleet: Fleet,
  node: NodeKey,
  upload: UploadId,
): Promise<Outcome<AttachmentRef>> {
  return attempt(() =>
    fleet.bridge.call(node, "attachment.finish", { upload }),
  );
}

/** A piece of a session's attachment from `offset`. */
export function readAttachment(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  attachment: AttachmentId,
  offset: number,
): Promise<Outcome<BlobChunk>> {
  return attempt(() =>
    fleet.bridge.call(node, "attachment.read", {
      session,
      attachment,
      offset,
    }),
  );
}
