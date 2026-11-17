// One machine's account reports on the page: read cached on open, explicit
// Refresh all / Refresh one, the lookup switch and a source's consent.
//
// [Decision Log] (2026-11-16, impl U, TRACKER 771)
// - Purpose: Paseo's usage queries (usage/queries.ts) without its query
//   library: the shown list stays on screen while a refresh runs and after
//   it fails (Paseo "the previous report stays on screen throughout").
// - Constraint: an answer for another machine, or after the page closed
//   the section, must never land; the node owns refresh admission, so the
//   page sends one request per click and never retries on its own.
// - Choice: a generation counter: `open` and `stop` start a new one, and
//   every answer is dropped unless its generation is still current.
//   A mutation (switch, consent) is followed by one cached read, because
//   the node may change several reports (Off clears snapshots).
import type { NodeKey } from "../../wire/bridge";
import type {
  ExternalLookup,
  LimitRefresh,
  UsageDisclosureVersion,
  UsageReportId,
  UsageReportLookup,
  UsageReportScope,
  UsageReports,
  UsageSourceDescriptor,
  UsageSourceId,
} from "../../wire/proto";
import type { Outcome, Problem } from "../outcome";
import { Store } from "../store";

export type AccountRead =
  | { kind: "idle" }
  | { kind: "loading"; node: NodeKey }
  | {
      kind: "ready";
      node: NodeKey;
      reports: UsageReports;
      /** Refresh all in flight; the list stays as it was. */
      refreshing: boolean;
      /** The last action that failed (a refresh, the switch, consent). */
      problem: Problem | null;
    }
  | { kind: "failed"; node: NodeKey; problem: Problem };

/** The typed call edge, injected so tests need no bridge. */
export interface AccountCalls {
  read: (
    node: NodeKey,
    refresh: LimitRefresh,
    scope: UsageReportScope,
  ) => Promise<Outcome<UsageReports>>;
  inspect: (
    node: NodeKey,
    id: UsageReportId,
  ) => Promise<Outcome<UsageReportLookup>>;
  lookup: (
    node: NodeKey,
    policy: ExternalLookup,
  ) => Promise<Outcome<ExternalLookup>>;
  source: (
    node: NodeKey,
    source: UsageSourceId,
    policy: ExternalLookup,
    disclosure: UsageDisclosureVersion,
  ) => Promise<Outcome<UsageSourceDescriptor>>;
}

/** The list with one report swapped for its refreshed copy, or removed. */
export function replaceReport(
  reports: UsageReports,
  id: UsageReportId,
  lookup: UsageReportLookup,
): UsageReports {
  if (lookup.kind === "found") {
    return {
      ...reports,
      cost_visibility: lookup.cost_visibility,
      reports: reports.reports.map((entry) =>
        entry.id === id ? lookup.report : entry,
      ),
    };
  }
  return {
    ...reports,
    reports: reports.reports.filter((entry) => entry.id !== id),
  };
}

export class AccountReader {
  readonly store = new Store<AccountRead>({ kind: "idle" });
  readonly #calls: AccountCalls;
  #generation = 0;
  #node: NodeKey | null = null;
  #scope: UsageReportScope = { kind: "machine" };

  constructor(calls: AccountCalls) {
    this.#calls = calls;
  }

  /** Show `node`'s reports (the machine's, or one session's account),
   * read cached (never contacts a source). */
  open(node: NodeKey, scope: UsageReportScope = { kind: "machine" }): void {
    this.#generation += 1;
    this.#node = node;
    this.#scope = scope;
    this.store.set({ kind: "loading", node });
    this.#read(this.#generation, node, "cached");
  }

  /** The section closed or the machine went away. */
  stop(): void {
    this.#generation += 1;
    this.#node = null;
    this.store.set({ kind: "idle" });
  }

  /** Paseo's Refresh all: every enabled source, explicitly (Operate). */
  refreshAll(): void {
    const shown = this.#shown();
    if (shown === null || shown.refreshing) return;
    this.store.set({ ...shown, refreshing: true, problem: null });
    this.#read(this.#generation, shown.node, "refresh");
  }

  /** One card's Refresh. */
  refreshOne(id: UsageReportId): void {
    const shown = this.#shown();
    if (shown === null) return;
    const generation = this.#generation;
    const node = shown.node;
    void this.#calls.inspect(node, id).then((outcome) => {
      if (generation !== this.#generation) return;
      const current = this.#shown();
      if (current === null) return;
      if (!outcome.ok) {
        this.store.set({ ...current, problem: outcome.problem });
      } else if (outcome.value.kind === "off") {
        this.#read(generation, node, "cached");
      } else {
        this.store.set({
          ...current,
          reports: replaceReport(current.reports, id, outcome.value),
          problem: null,
        });
      }
    });
  }

  /** The machine-wide switch (Admin); the list is read again after it. */
  setLookup(policy: ExternalLookup): void {
    const node = this.#node;
    if (node === null) return;
    const generation = this.#generation;
    void this.#calls.lookup(node, policy).then((outcome) => {
      this.#afterMutation(generation, node, outcome);
    });
  }

  /** One source's consent (Admin), against the disclosure shown. */
  setSource(
    source: UsageSourceId,
    policy: ExternalLookup,
    disclosure: UsageDisclosureVersion,
  ): void {
    const node = this.#node;
    if (node === null) return;
    const generation = this.#generation;
    void this.#calls
      .source(node, source, policy, disclosure)
      .then((outcome) => {
        this.#afterMutation(generation, node, outcome);
      });
  }

  #afterMutation(
    generation: number,
    node: NodeKey,
    outcome: Outcome<unknown>,
  ): void {
    if (generation !== this.#generation) return;
    if (outcome.ok) {
      this.#read(generation, node, "cached");
      return;
    }
    const current = this.store.get();
    if (current.kind === "ready") {
      this.store.set({ ...current, problem: outcome.problem });
    } else {
      this.store.set({ kind: "failed", node, problem: outcome.problem });
    }
  }

  #shown(): Extract<AccountRead, { kind: "ready" }> | null {
    const current = this.store.get();
    return current.kind === "ready" ? current : null;
  }

  #read(generation: number, node: NodeKey, refresh: LimitRefresh): void {
    void this.#calls.read(node, refresh, this.#scope).then((outcome) => {
      if (generation !== this.#generation) return;
      const shown = this.#shown();
      if (outcome.ok) {
        this.store.set({
          kind: "ready",
          node,
          reports: outcome.value,
          refreshing: false,
          problem: null,
        });
      } else if (shown !== null) {
        // A failed refresh keeps the list that was on screen.
        this.store.set({
          ...shown,
          refreshing: false,
          problem: outcome.problem,
        });
      } else {
        this.store.set({ kind: "failed", node, problem: outcome.problem });
      }
    });
  }
}
