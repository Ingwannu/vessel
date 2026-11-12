// Drafts (DESIGN.md §6.15): what is typed into a composer and not yet
// sent, one per session and one for the new-session page, kept by the
// bridge (ADR 0028) so it survives leaving the page, a reload and a new
// launch.
//
// [Decision Log]
// - Purpose: all three references keep a draft per session; the page's
//   origin is new on every launch, so the browser cannot keep it.
// - Constraint: never a write per keystroke to the bridge, nothing read on
//   a render path, and a tab's unsaved text never replaced by a read.
// - Choice: the text is kept here on every key (memory, synchronous, so a
//   composer that comes back shows it at once), and saved at most once
//   per `SAVE_MS` of typing (GOAT debounces 300 ms, Paseo throttles
//   200 ms, both to local storage; the bridge is a socket away), at once
//   when the page is hidden or closed, when a composer is left, and when
//   the text is sent (then empty). All drafts are read when the socket
//   opens; one is read again when its composer opens and nothing has been
//   typed into it in this tab, so another tab's newer text shows. Unsaved
//   text is saved again after a reconnect.
// - Bound: 128 KiB of UTF-8 per draft, the bridge's (agent-orchestrator's
//   message limit). A longer one stays in this tab's memory and the
//   composer says it is not kept.

import type { DraftKey, OpResult } from "../wire/bridge";
import type { Fleet } from "./fleet";
import { Store } from "./store";

/** How long typing settles before a draft is saved. */
export const SAVE_MS = 500;

/** The longest draft the bridge keeps, in UTF-8 bytes. */
export const MAX_DRAFT_BYTES = 128 * 1024;

/** A draft's key as text, for maps. */
export function draftId(key: DraftKey): string {
  switch (key.kind) {
    case "new":
      return "new";
    case "session":
      return `session:${key.session}`;
  }
}

/** `text` with any lone surrogate (half of a character an edit cut)
 * replaced, as `String.prototype.toWellFormed` would: the bridge decodes
 * UTF-8 and would refuse the whole frame. */
export function wellFormed(text: string): string {
  return text.replace(
    /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/g,
    "\uFFFD",
  );
}

/** Whether `text` fits the bridge's bound. */
export function fits(text: string): boolean {
  return new TextEncoder().encode(text).length <= MAX_DRAFT_BYTES;
}

/** One draft as this tab holds it. */
interface Held {
  key: DraftKey;
  text: string;
  /** Typed here and not yet saved. */
  dirty: boolean;
  /** Typed into in this tab (a read then never replaces it). */
  touched: boolean;
}

/** What a composer needs to know of its draft beyond the text. */
export type DraftNote = "kept" | "too-long";

/** The page's drafts. */
export class Drafts {
  readonly #fleet: Fleet;
  readonly #held = new Map<string, Held>();
  readonly #timers = new Map<string, ReturnType<typeof setTimeout>>();
  /** Changes whenever a draft's text does (typed here, or read): what a
   * composer subscribes to. Only composers do, so a key re-renders them
   * alone. */
  readonly version = new Store(0);

  constructor(fleet: Fleet) {
    this.#fleet = fleet;
  }

  /** The text of `key`'s draft, as this tab holds it. */
  text(key: DraftKey): string {
    return this.#held.get(draftId(key))?.text ?? "";
  }

  /** Whether `key`'s draft is kept by the bridge or only here. */
  note(key: DraftKey): DraftNote {
    return fits(this.text(key)) ? "kept" : "too-long";
  }

  /** The person typed: keep it, and save it once typing settles. */
  type(key: DraftKey, text: string): void {
    const id = draftId(key);
    this.#held.set(id, { key, text, dirty: true, touched: true });
    this.version.update((count) => count + 1);
    const timer = this.#timers.get(id);
    if (timer !== undefined) {
      clearTimeout(timer);
    }
    this.#timers.set(
      id,
      setTimeout(() => {
        this.#timers.delete(id);
        this.#save(id);
      }, SAVE_MS),
    );
  }

  /** Save every unsaved draft now: the page is going, a composer is
   * left, or a send is about to empty one. */
  flush(): void {
    for (const [id, timer] of this.#timers) {
      clearTimeout(timer);
      this.#timers.delete(id);
    }
    for (const id of this.#held.keys()) {
      this.#save(id);
    }
  }

  /** Empty `key`'s draft, once what it held was sent. */
  clear(key: DraftKey): void {
    this.type(key, "");
    this.flush();
  }

  /** Read every draft (the socket opened); keep this tab's own unsaved
   * text, and save it again. */
  async readAll(): Promise<void> {
    const result = await this.#ask({ op: "drafts" });
    if (result?.op !== "drafts") {
      return;
    }
    for (const draft of result.drafts.drafts) {
      const id = draftId(draft.key);
      if (this.#held.get(id)?.touched !== true) {
        this.#held.set(id, {
          key: draft.key,
          text: draft.text,
          dirty: false,
          touched: false,
        });
      }
    }
    this.flush();
    this.version.update((count) => count + 1);
  }

  /** Read `key`'s draft again (its composer opened), unless this tab
   * typed into it. */
  async readOne(key: DraftKey): Promise<void> {
    const id = draftId(key);
    if (this.#held.get(id)?.touched === true) {
      return;
    }
    const result = await this.#ask({ op: "draft", key });
    if (result?.op !== "draft" || this.#held.get(id)?.touched === true) {
      return;
    }
    const text = result.text ?? "";
    if (text !== this.text(key)) {
      this.#held.set(id, { key, text, dirty: false, touched: false });
      this.version.update((count) => count + 1);
    }
  }

  #save(id: string): void {
    const held = this.#held.get(id);
    if (held === undefined || !held.dirty || !fits(held.text)) {
      return;
    }
    held.dirty = false;
    // A lone surrogate (half of an emoji cut by an edit) would make the
    // frame unreadable; the bridge keeps well-formed text.
    const text = wellFormed(held.text);
    void this.#ask({ op: "save_draft", key: held.key, text }).then((result) => {
      if (result === null) {
        held.dirty = true;
      }
    });
  }

  async #ask(
    op:
      | { op: "drafts" }
      | { op: "draft"; key: DraftKey }
      | { op: "save_draft"; key: DraftKey; text: string },
  ): Promise<OpResult | null> {
    try {
      return await this.#fleet.bridge.op(op);
    } catch {
      return null;
    }
  }
}
