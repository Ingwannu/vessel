// One retained send attempt, not a client queue. Unknown attempts never retry,
// lose their blobs through LRU eviction, or allocate another message identity.
import type { SubmitCheck } from "../wire/bridge";
import type {
  ClientMessageId,
  ContentBlock,
  NodeId,
  SessionId,
  SubmissionDelivery,
  SubmitPrompt,
  SubmitPromptReply,
} from "../wire/proto";
import { MAX_DRAFT_BYTES } from "./drafts";
import { promptReceiptFits } from "./submission-receipt";
import type { Outcome } from "./outcome";
import type { TrayItem } from "./tray-item";
import { uploadCaptured, type PreparedFile } from "./prepared-files";
export type { PreparedFile } from "./prepared-files";
import type { Begun, UploadCalls } from "./upload";

export const MAX_PREPARED_SUBMISSIONS = 16;
export const MAX_PREPARED_BYTES = 128 * 1024 * 1024;
const RETAINED_ROW_BYTES = 4096;
const MAX_PREPARED_FILES = 16;

export interface PreparedHandle {
  readonly message: ClientMessageId;
}
export interface PreparedInput {
  node: NodeId;
  session: SessionId;
  message: ClientMessageId;
  text: string;
  delivery: SubmissionDelivery;
  files: readonly TrayItem[];
}
export interface PreparedPort {
  check: (request: SubmitPrompt) => Promise<Outcome<SubmitCheck>>;
  uploads: UploadCalls;
  submit: (request: SubmitPrompt) => Promise<Outcome<SubmitPromptReply>>;
}
export type PreparedEnd =
  | { status: "unsupported" }
  | {
      status: "not_admitted";
      reply: Extract<
        SubmitPromptReply,
        { status: "capacity" | "not_admitted" | "refused" }
      >;
      files: readonly PreparedFile[];
    }
  | { status: "conflict"; message: ClientMessageId }
  | {
      status: "preparation_failed";
      message: string;
      files: readonly PreparedFile[];
    }
  | {
      status: "accepted";
      reply: Extract<SubmitPromptReply, { status: "accepted" }>;
      files: readonly PreparedFile[];
    }
  | { status: "uncertain"; message: ClientMessageId };
export type PreparedReservation =
  | { status: "reserved"; handle: PreparedHandle }
  | { status: "held"; message: ClientMessageId }
  | { status: "refused"; reason: string };
interface Retained {
  handle: PreparedHandle;
  key: string;
  charge: number;
  input: PreparedInput;
  files: PreparedFile[];
  resumes: Map<string, Begun>;
  request: SubmitPrompt;
  phase:
    | "reserved"
    | "preparing"
    | "preparation_failed"
    | "dispatching"
    | "uncertain";
}

export class PreparedSubmissions {
  readonly #owners = new Map<string, Retained>();
  readonly #handles = new WeakMap<PreparedHandle, Retained>();
  #bytes = 0;

  reserve(input: PreparedInput): PreparedReservation {
    const key = `${input.node}\u0000${input.session}`;
    const held = this.#owners.get(key);
    if (held !== undefined)
      return { status: "held", message: held.handle.message };
    const encoded = new TextEncoder();
    const textBytes = encoded.encode(input.text).length;
    if (
      input.files.some(
        (item) => !Number.isSafeInteger(item.file.size) || item.file.size < 0,
      )
    )
      return {
        status: "refused",
        reason: "An attachment has an invalid size.",
      };
    const charge = input.files.reduce(
      (sum, item) =>
        sum +
        item.file.size +
        encoded.encode(item.name + item.mime + item.key).length +
        RETAINED_ROW_BYTES,
      textBytes +
        encoded.encode(input.node + input.session + input.message).length +
        RETAINED_ROW_BYTES,
    );
    if (
      textBytes > MAX_DRAFT_BYTES ||
      input.files.length > MAX_PREPARED_FILES ||
      !Number.isSafeInteger(charge) ||
      this.#owners.size >= MAX_PREPARED_SUBMISSIONS ||
      this.#bytes + charge > MAX_PREPARED_BYTES
    )
      return {
        status: "refused",
        reason:
          "Prepared messages have reached this tab's retained-data limit.",
      };
    if (
      input.files.some(
        (item) => item.state.kind !== "attached" && item.state.kind !== "ready",
      )
    )
      return {
        status: "refused",
        reason: "An attachment is not ready for preparation.",
      };
    if (
      input.files.some(
        (item) => item.state.kind === "ready" && item.state.node !== input.node,
      )
    )
      return {
        status: "refused",
        reason: "An attachment belongs to another machine. Attach it again.",
      };
    const handle: PreparedHandle = Object.freeze({ message: input.message });
    const retained: Retained = {
      handle,
      key,
      charge,
      input: {
        ...input,
        files: input.files.map((item) => ({
          ...item,
          state:
            item.state.kind === "ready"
              ? { ...item.state, ref: { ...item.state.ref } }
              : { ...item.state },
        })),
      },
      files: [],
      resumes: new Map(),
      phase: "reserved",
      request: {
        session: input.session,
        message: input.message,
        content: textContent(input.text),
        delivery: input.delivery,
      },
    };
    this.#owners.set(key, retained);
    this.#handles.set(handle, retained);
    this.#bytes += charge;
    return { status: "reserved", handle };
  }

  /** A handle can dispatch once; uncertainty never authorizes another call. */
  async run(handle: PreparedHandle, port: PreparedPort): Promise<PreparedEnd> {
    const retained = this.#handles.get(handle);
    if (
      retained === undefined ||
      (retained.phase !== "reserved" && retained.phase !== "preparation_failed")
    )
      return { status: "uncertain", message: handle.message };
    retained.phase = "preparing";
    try {
      const check = await port.check(structuredClone(retained.request));
      if (!check.ok) return this.#refused(retained, check.problem.message);
      if (check.value.status === "unsupported") {
        if (retained.files.length > 0 || retained.resumes.size > 0)
          return this.#refused(
            retained,
            "Support changed after attachment preparation began.",
          );
        this.#release(retained);
        return { status: "unsupported" };
      }
      if (
        check.value.status !== "ready" ||
        check.value.node !== retained.input.node
      )
        return this.#refused(
          retained,
          "The prepared recipient is unavailable or changed.",
        );
      const uploadFailure = await uploadCaptured(
        retained.input.files,
        retained.files,
        retained.resumes,
        port.uploads,
      );
      if (uploadFailure !== null) return this.#refused(retained, uploadFailure);
      retained.request.content = [
        ...textContent(retained.input.text),
        ...retained.files.map(({ ref }): ContentBlock => ({
          type: "attachment",
          attachment: ref,
        })),
      ];
      retained.phase = "dispatching";
      const outcome = await port.submit(structuredClone(retained.request));
      if (!outcome.ok) return this.#uncertain(retained);
      const reply = outcome.value;
      const message =
        reply.status === "accepted" ? reply.receipt.message : reply.message;
      if (message !== retained.handle.message) return this.#uncertain(retained);
      switch (reply.status) {
        case "accepted":
          if (!promptReceiptFits(retained.input.delivery, reply))
            return this.#uncertain(retained);
          this.#release(retained);
          return { status: "accepted", reply, files: retained.files };
        case "capacity":
        case "not_admitted":
        case "refused":
          this.#release(retained);
          return { status: "not_admitted", reply, files: retained.files };
        case "key_conflict":
          retained.phase = "uncertain";
          return { status: "conflict", message: retained.handle.message };
        case "outcome_unknown":
          return this.#uncertain(retained);
      }
    } catch {
      return retained.phase === "dispatching"
        ? this.#uncertain(retained)
        : this.#refused(retained, "Message preparation did not finish.");
    }
  }
  /** Only an explicitly abandoned pre-dispatch attempt may release its lease. */
  abandon(handle: PreparedHandle): boolean {
    const retained = this.#handles.get(handle);
    if (
      retained === undefined ||
      (retained.phase !== "reserved" && retained.phase !== "preparation_failed")
    )
      return false;
    this.#release(retained);
    return true;
  }
  /** Recover the same retained owner after a composer unmounts; no resend occurs. */
  held(node: NodeId, session: SessionId): PreparedHandle | undefined {
    return this.#owners.get(`${node}\u0000${session}`)?.handle;
  }
  #uncertain(retained: Retained): PreparedEnd {
    retained.phase = "uncertain";
    return { status: "uncertain", message: retained.handle.message };
  }
  #refused(retained: Retained, message: string): PreparedEnd {
    retained.phase = "preparation_failed";
    return {
      status: "preparation_failed",
      message,
      files: Object.freeze(retained.files.slice()),
    };
  }
  #release(retained: Retained): void {
    this.#owners.delete(retained.key);
    this.#handles.delete(retained.handle);
    this.#bytes -= retained.charge;
  }
}
function textContent(text: string): ContentBlock[] {
  return text === "" ? [] : [{ type: "text", text }];
}
