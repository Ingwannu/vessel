// One active ledger read plus one latest selection, not a request for
// every intermediate click. An old response cannot become another
// machine's result; closing the page discards queued work, not DB work
// already admitted by the node (which owns its cancellation-safe bound).
import type { NodeKey } from "../../wire/bridge";
import type { UsageReport } from "../../wire/proto";
import type { Outcome, Problem } from "../outcome";
import { Store } from "../store";
import type { UsageRange } from "./calls";
import {
  copiedScope,
  MACHINE_USAGE,
  usageNames,
  usageScopeKey,
  type RollingUsageScope,
  type UsageNames,
} from "./scope";

export interface UsageSelection {
  node: NodeKey;
  period: UsageRange;
  scope?: RollingUsageScope;
}

export type UsageRead =
  | { kind: "idle" }
  | { kind: "loading"; selection: UsageSelection }
  | { kind: "ready"; selection: UsageSelection; report: UsageReport }
  | { kind: "failed"; selection: UsageSelection; problem: Problem };

type LoadUsage = (selection: UsageSelection) => Promise<Outcome<UsageReport>>;

/** Coalesces visible selections; the caller supplies the existing typed call edge. */
export class UsageReader {
  readonly store = new Store<UsageRead>({ kind: "idle" });
  readonly names = new Store<UsageNames | null>(null);
  readonly #load: LoadUsage;
  #selected: UsageSelection | null = null;
  #next: UsageSelection | null = null;
  #reading = false;

  constructor(load: LoadUsage) {
    this.#load = load;
  }

  /** Refresh, or select another machine/interval; intermediate queued choices are replaced. */
  request(selection: UsageSelection): void {
    // Every invocation gets a private identity, including callers that reuse
    // one selection object after stop or while refreshing the same scope.
    const request = {
      ...selection,
      ...(selection.scope === undefined
        ? {}
        : { scope: copiedScope(selection.scope) }),
    };
    if (this.names.get()?.node !== request.node) this.names.set(null);
    this.#selected = request;
    this.#next = request;
    this.store.set({ kind: "loading", selection: request });
    if (!this.#reading) {
      this.#reading = true;
      void this.#drain();
    }
  }

  /** A mounted reader may be reused; no result from its previous lifetime can publish. */
  stop(): void {
    this.#selected = null;
    this.#next = null;
    this.store.set({ kind: "idle" });
    this.names.set(null);
  }

  async #drain(): Promise<void> {
    try {
      while (this.#next !== null) {
        const selection = this.#next;
        this.#next = null;
        const outcome = await this.#load(selection);
        if (this.#selected === selection) {
          if (
            outcome.ok &&
            (selection.scope ?? MACHINE_USAGE).kind === "machine"
          ) {
            this.names.set(usageNames(selection.node, outcome.value));
          }
          this.store.set(
            outcome.ok
              ? { kind: "ready", selection, report: outcome.value }
              : { kind: "failed", selection, problem: outcome.problem },
          );
        }
      }
    } finally {
      this.#reading = false;
    }
  }
}

/** Render selection changes as loading before their effect can start the next read. */
export function usageFor(
  read: UsageRead,
  selection: UsageSelection,
): UsageRead {
  switch (read.kind) {
    case "idle":
      return read;
    case "loading":
    case "ready":
    case "failed":
      return read.selection.node === selection.node &&
        read.selection.period === selection.period &&
        usageScopeKey(read.selection.scope ?? MACHINE_USAGE) ===
          usageScopeKey(selection.scope ?? MACHINE_USAGE)
        ? read
        : { kind: "loading", selection };
  }
}
