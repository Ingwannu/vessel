// The files attached to one composer, and their uploads (milestone 11,
// Track A-page): picked, pasted or dropped, checked against what the
// agent takes and the node's limits as they come, sent when the prompt
// is, each with its progress, failure, retry and removal.
//
// [Decision Log]
// - Purpose: agent-orchestrator, GOAT and Paseo attach files to the next
//   message, show each in a strip over the text, and send them with it.
//   Vessel's node keeps attachments with the session (ADR 0025), so a
//   file must be uploaded before the prompt that names it.
// - Constraint: an upload needs a live session whose agent has said what
//   it takes (images only after its `ready`), and the limits come from
//   `session.conversation`; the new-session page has neither until the
//   session is created. The node has no call to give an upload or an
//   attachment back: whatever is begun counts against the session's
//   100 MiB until the session goes.
// - Alternatives: uploading as soon as a file is attached
//   (agent-orchestrator's), which shows progress early but spends the
//   session's room on files the person then removes.
// - Choice: a file is checked when it is attached (or, with no session
//   yet, once there is one) and uploaded when Send is pressed, at most two
//   at a time; the prompt goes when every file is in. A refused file, or
//   one whose upload failed, stops Send until it is removed or retried; a
//   retry goes on from what the node has. Trays are kept per session for
//   the tab's life (16 at most), so leaving a session and coming back
//   finds its files; nothing is written anywhere (a file cannot be).
// - Cost: Send waits for the uploads; a pasted image (a blob the browser
//   holds) stays in memory until it is sent or removed. A chip's picture
//   is a small `data:` thumbnail drawn from the file (the page's content
//   policy takes no `blob:` address, data/thumbnail.ts).

import type { AttachmentRef, NodeId } from "../wire/proto";
import { attachmentName, kindOf, mimeOf, refusal, refusalLine } from "./attach";
import { Store } from "./store";
import { thumbnail } from "./thumbnail";
import {
  checked,
  type ItemState,
  type TrayItem,
  type TrayReady,
  type TrayTarget,
  trayReady,
} from "./tray-item";
import { type Begun, send, type UploadCalls } from "./upload";

export type { TrayTarget } from "./tray-item";

/** Files uploading at once from one composer (the node allows a caller
 * four; two leaves room for another tab). */
const AT_ONCE = 2;

/** A chip's thumbnail is 24px; drawn at twice that. */
const CHIP_THUMB = 48;

let counter = 0;

export class Tray {
  readonly items = new Store<readonly TrayItem[]>([]);
  #target: TrayTarget | null = null;
  /** Keys whose upload was asked to stop. */
  readonly #stopping = new Set<string>();

  /** Attach `files`, each checked as it comes. */
  add(files: readonly File[]): void {
    const added = files.map((file): TrayItem => {
      const mime = mimeOf(file.type);
      const kind = kindOf(mime);
      counter += 1;
      return {
        key: `f${String(counter)}`,
        name: attachmentName(file.name, kind === "image" ? "image" : "file"),
        mime,
        kind,
        size: file.size,
        file,
        preview: null,
        state: { kind: "held" },
      };
    });
    this.items.update((items) => [...items, ...added]);
    for (const item of added) {
      if (item.kind === "image") {
        void thumbnail(item.file, CHIP_THUMB).then((preview) => {
          this.items.update((items) =>
            items.map((candidate) =>
              candidate.key === item.key
                ? { ...candidate, preview }
                : candidate,
            ),
          );
        });
      }
    }
    this.#settle();
  }

  /** Where the files go from now; `null` while there is nowhere. */
  target(target: TrayTarget | null): void {
    this.#target = target;
    if (target !== null) {
      // The agent may take now what it did not (it said what it takes
      // since): look at refused files again.
      this.items.update((items) =>
        items.map((item) =>
          item.state.kind === "refused"
            ? { ...item, state: { kind: "held" } }
            : item,
        ),
      );
    }
    this.#settle();
  }

  /** Take a file out, stopping its upload. */
  remove(key: string): void {
    const item = this.items.get().find((candidate) => candidate.key === key);
    if (item === undefined) {
      return;
    }
    this.#stopping.add(key);
    this.items.update((items) =>
      items.filter((candidate) => candidate.key !== key),
    );
    this.#settle();
  }

  /** Upload a failed file again, from what the node has. */
  retry(key: string): void {
    this.items.update((items) =>
      items.map((item) =>
        item.key === key &&
        item.state.kind === "failed" &&
        item.state.again === "yes"
          ? { ...item, state: { kind: "waiting", resume: item.state.resume } }
          : item,
      ),
    );
    this.#settle();
  }

  /** Upload what is attached, then answer whether the prompt can go. */
  async upload(): Promise<TrayReady> {
    this.items.update((items) =>
      items.map((item) =>
        item.state.kind === "attached"
          ? { ...item, state: { kind: "waiting", resume: null } }
          : item,
      ),
    );
    this.#settle();
    const busy = () =>
      this.items
        .get()
        .some(
          (item) =>
            item.state.kind === "waiting" || item.state.kind === "uploading",
        );
    while (busy()) {
      await new Promise<void>((resolve) => {
        const stop = this.items.subscribe(() => {
          stop();
          resolve();
        });
      });
    }
    return this.ready();
  }

  /** Whether the files can go with a prompt. */
  ready(): TrayReady {
    return trayReady(this.items.get(), this.#target?.node);
  }

  /** Empty the tray once its files went with a prompt; its images are
   * handed to `keep` (the timeline shows them from here, not from the
   * machine). */
  sent(keep: (ref: AttachmentRef, image: Blob) => void): void {
    for (const item of this.items.get()) {
      if (item.state.kind === "ready" && item.kind === "image") {
        keep(item.state.ref, item.file);
      }
    }
    this.items.set([]);
  }

  /** Drop every file (the tray is forgotten). */
  dispose(): void {
    for (const item of this.items.get()) {
      this.#stopping.add(item.key);
    }
    this.items.set([]);
  }

  #set(key: string, state: ItemState): void {
    this.items.update((items) =>
      items.map((item) => (item.key === key ? { ...item, state } : item)),
    );
  }

  /** Check what is held, and start what waits, two at a time. */
  #settle(): void {
    const target = this.#target;
    if (target === null) {
      return;
    }
    const kept: TrayItem[] = [];
    const next = this.items.get().map((item): TrayItem => {
      if (item.state.kind !== "held") {
        if (item.state.kind !== "refused") {
          kept.push(item);
        }
        return item;
      }
      const refused = refusal(item, kept, target.abilities, target.limits);
      if (refused !== null) {
        const reason = refusalLine(refused, target.agent);
        return { ...item, state: { kind: "refused", reason } };
      }
      kept.push(item);
      return { ...item, state: { kind: "attached" } };
    });
    this.items.set(next);
    let running = next.filter((item) => item.state.kind === "uploading").length;
    for (const item of next) {
      if (running >= AT_ONCE) {
        break;
      }
      if (item.state.kind === "waiting") {
        running += 1;
        this.#upload(item, item.state.resume, target.calls, target.node);
      }
    }
  }

  #upload(
    item: TrayItem,
    resume: Begun | null,
    calls: UploadCalls,
    node?: NodeId,
  ): void {
    this.#stopping.delete(item.key);
    this.#set(item.key, { kind: "uploading", received: resume?.received ?? 0 });
    const stopped = () => this.#stopping.has(item.key);
    void checked(item).then(async (problem) => {
      if (problem !== null) {
        this.#set(item.key, {
          kind: "failed",
          reason: problem,
          again: "never",
          resume: null,
        });
        this.#settle();
        return;
      }
      const end = await send(
        calls,
        item.file,
        item.name,
        item.mime,
        resume,
        (received) => {
          if (!stopped()) {
            this.#set(item.key, { kind: "uploading", received });
          }
        },
        stopped,
      );
      this.#stopping.delete(item.key);
      switch (end.kind) {
        case "done":
          this.#set(item.key, {
            kind: "ready",
            ref: end.ref,
            ...(node === undefined ? {} : { node }),
          });
          break;
        case "failed":
          this.#set(item.key, {
            kind: "failed",
            reason: end.problem.message,
            again: end.again,
            resume: end.resume,
          });
          break;
        case "stopped":
          break;
      }
      this.#settle();
    });
  }
}
