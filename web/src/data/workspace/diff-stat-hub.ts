// The sidebar card's diff stats, one long poll per connected machine.
//
// [Decision Log]
// - Purpose: `workspace.list` serves the count (ADR 0120). The registry
//   snapshot does not carry it, and a save must not wake that poll.
// - Constraint: an older node ignores `changed_from` and answers at
//   once, so a loop that sent it would spin. The poll runs only while a
//   surface holds the hub.
// - Choice: ask only when the constant is a number and the peer speaks
//   at least that number. The first read omits `changed_from`. Later
//   reads send the generation already shown, and an empty answer keeps
//   it. Each answer then waits out `QUIET_MS`, as the label hub does.

import type { Bridge } from "../../bridge/client";
import type { NodeKey, NodeStatus } from "../../wire/bridge";
import type { DiffStat } from "../../wire/proto";
import { DIFF_STAT_PROTOCOL } from "../../wire/proto";
import type { Fleet, FleetState } from "../fleet";
import { attempt, type Outcome } from "../outcome";
import { entriesOf } from "../records";
import { Store } from "../store";
import {
  diffStatChangedFrom,
  diffStatServed,
  nextStatRevision,
  statsByWorkspace,
} from "./diff-stat";
import { pause, quiet } from "./pace";

const RETRY_MS = 5000;

/** Counts by workspace id. Null hides the card's diff row. */
export type NodeDiffStats = Readonly<Record<string, DiffStat | null>>;

export interface DiffStatState {
  byNode: Readonly<Record<NodeKey, NodeDiffStats>>;
}

const EMPTY: DiffStatState = { byNode: {} };

/** What the hub reads: link state and calls (a `Fleet`, or a test). */
export interface DiffStatSource {
  readonly store: {
    get: () => Pick<FleetState, "statuses">;
    subscribe: (listener: () => void) => () => void;
  };
  readonly bridge: Pick<Bridge, "call">;
}

function peerProtocol(status: NodeStatus | undefined): number | null {
  return status?.state === "connected" ? status.protocol : null;
}

function unsupported(outcome: Outcome<unknown>): boolean {
  return (
    !outcome.ok &&
    outcome.problem.from === "call" &&
    outcome.problem.failure.kind === "not_in_protocol"
  );
}

function same(held: NodeDiffStats | undefined, next: NodeDiffStats): boolean {
  if (held === undefined) {
    return false;
  }
  const keys = Object.keys(next);
  if (keys.length !== Object.keys(held).length) {
    return false;
  }
  return keys.every((key) => {
    const left = held[key];
    const right = next[key];
    if (left == null || right == null) {
      return left === right;
    }
    return (
      left.additions === right.additions && left.deletions === right.deletions
    );
  });
}

export class WorkspaceDiffStats {
  readonly store = new Store<DiffStatState>(EMPTY);
  readonly #fleet: DiffStatSource;
  #holders = 0;
  readonly #running = new Map<NodeKey, symbol>();
  #unsubscribe: (() => void) | null = null;

  constructor(fleet: DiffStatSource) {
    this.#fleet = fleet;
  }

  /** Keep the counts current until the returned function is called. */
  hold(): () => void {
    this.#holders += 1;
    if (this.#holders === 1) {
      this.#unsubscribe = this.#fleet.store.subscribe(() => {
        this.#follow();
      });
      this.#follow();
    }
    let released = false;
    return () => {
      if (released) {
        return;
      }
      released = true;
      this.#holders -= 1;
      if (this.#holders === 0) {
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#running.clear();
      }
    };
  }

  #follow(): void {
    const statuses = this.#fleet.store.get().statuses;
    for (const [node, status] of entriesOf(statuses)) {
      const protocol = peerProtocol(status);
      const served =
        protocol !== null && diffStatServed(DIFF_STAT_PROTOCOL, protocol);
      if (served && !this.#running.has(node)) {
        const run = Symbol(node);
        this.#running.set(node, run);
        void this.#loop(node, run);
      } else if (!served && this.#running.has(node)) {
        this.#running.delete(node);
      }
    }
  }

  #alive(node: NodeKey, run: symbol): boolean {
    return this.#running.get(node) === run;
  }

  async #loop(node: NodeKey, run: symbol): Promise<void> {
    let revision: number | null = null;
    while (this.#alive(node, run)) {
      const protocol = peerProtocol(this.#fleet.store.get().statuses[node]);
      if (protocol === null || !diffStatServed(DIFF_STAT_PROTOCOL, protocol)) {
        if (!this.#alive(node, run)) {
          return;
        }
        await pause(RETRY_MS, null);
        continue;
      }
      const asked = performance.now();
      const changed = diffStatChangedFrom(
        DIFF_STAT_PROTOCOL,
        protocol,
        revision,
      );
      const outcome = await attempt(() =>
        this.#fleet.bridge.call(node, "workspace.list", {
          project: null,
          lifecycle: "active",
          visibility: "foreground",
          ...(changed === null ? {} : { changed_from: changed }),
        }),
      );
      if (!this.#alive(node, run)) {
        return;
      }
      if (outcome.ok) {
        const rows = outcome.value;
        revision = nextStatRevision(revision, rows);
        const next: Record<string, DiffStat | null> = {};
        for (const [id, stat] of statsByWorkspace(rows)) {
          next[id] = stat;
        }
        this.store.update((state) =>
          same(state.byNode[node], next)
            ? state
            : { byNode: { ...state.byNode, [node]: next } },
        );
        await quiet(asked, null);
      } else if (unsupported(outcome)) {
        return;
      } else {
        revision = null;
        await pause(RETRY_MS, null);
      }
    }
  }
}

const HUBS = new WeakMap<Fleet, WorkspaceDiffStats>();

/** The one hub of `fleet`, made on first use. */
export function diffStatsOf(fleet: Fleet): WorkspaceDiffStats {
  let hub = HUBS.get(fleet);
  if (hub === undefined) {
    hub = new WorkspaceDiffStats(fleet);
    HUBS.set(fleet, hub);
  }
  return hub;
}
