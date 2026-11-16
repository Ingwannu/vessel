// Each connected machine's workspace registry and label catalog, kept
// current while a surface shows them (the sidebar grouped by workspace,
// the manager), and the label calls.
//
// [Decision Log]
// - Purpose: the sidebar's workspace rows, their chips, the picker and
//   the manager all read the same two things per machine: the registry
//   snapshot (workspaces, memberships and assigned names, protocol 25)
//   and the label catalog (protocol 28).
// - Reference: Paseo keeps one label replica per host, fed by a list and
//   then pushed updates, and refetches on a gap; its workspace list comes
//   from the host's own stream.
// - Choice: two long polls per connected machine, the registry by revision
//   (`changed_from`) and the catalog by cursor (`read: observe`), each
//   restarting with a fresh read after a failure, after a short pause.
//   They run only while something holds the hub (`hold`): a tab has 64
//   calls at once (vessel-web `MAX_CALLS`), and two waiting calls per
//   machine are not free when nothing draws labels.
// - A machine older than either call is "unsupported": it is asked once
//   per connection, not again. Mutations do not patch the replicas: the
//   node wakes both long polls when it commits (ADR 0086), so the page
//   shows what the node holds.

import type { NodeKey } from "../../wire/bridge";
import type {
  Label,
  LabelColor,
  RegistrySnapshot,
  WorkspaceId,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";
import { entriesOf } from "../records";
import { Store } from "../store";
import { CatalogReplica } from "./catalog";

const RETRY_MS = 5000;

/** One machine's labels as the page holds them. */
export interface MachineLabels {
  state: "loading" | "online" | "offline" | "unsupported";
  labels: readonly Label[];
  /** The last read's failure, kept until a read succeeds. */
  error: string | null;
}

export interface LabelsState {
  registries: Readonly<Record<NodeKey, RegistrySnapshot>>;
  catalogs: Readonly<Record<NodeKey, MachineLabels>>;
}

const EMPTY: LabelsState = { registries: {}, catalogs: {} };

function unsupported(outcome: Outcome<unknown>): boolean {
  return (
    !outcome.ok &&
    outcome.problem.from === "call" &&
    outcome.problem.failure.kind === "not_in_protocol"
  );
}

/** Whether `next` shows nothing `held` does not. A wait that ran out
 * answers the same revision; derived memberships are not revisioned (ADR
 * 0106), so those are compared too before the sidebar draws again. */
function sameRegistry(
  held: RegistrySnapshot | undefined,
  next: RegistrySnapshot,
): boolean {
  return (
    held?.revision === next.revision &&
    JSON.stringify(held.memberships) === JSON.stringify(next.memberships)
  );
}

function pause(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export class WorkspaceLabels {
  readonly store = new Store<LabelsState>(EMPTY);
  readonly #fleet: Fleet;
  #holders = 0;
  /** The machines with loops running, by the connection they run for. */
  readonly #running = new Map<NodeKey, symbol>();
  #unsubscribe: (() => void) | null = null;

  constructor(fleet: Fleet) {
    this.#fleet = fleet;
  }

  /** Keep the replicas current until the returned function is called. */
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

  /** Assign `label` to `workspace` (creating it in the catalog when it is
   * new there), or take it off. */
  assign(
    node: NodeKey,
    workspace: WorkspaceId,
    label: Label,
    assigned: boolean,
  ): Promise<Outcome<unknown>> {
    return attempt(() =>
      this.#fleet.bridge.call(node, "workspace.label.assign", {
        workspace,
        label,
        assignment: assigned ? "assigned" : "unassigned",
      }),
    );
  }

  update(
    node: NodeKey,
    name: string,
    change: { new_name?: string; color?: LabelColor },
  ): Promise<Outcome<unknown>> {
    return attempt(() =>
      this.#fleet.bridge.call(node, "workspace.label.update", {
        name,
        ...change,
      }),
    );
  }

  /** How many workspaces carry `name` now. */
  async inspect(node: NodeKey, name: string): Promise<Outcome<number>> {
    const outcome = await attempt(() =>
      this.#fleet.bridge.call(node, "workspace.label.inspect_delete", { name }),
    );
    return outcome.ok ? { ok: true, value: outcome.value.affected } : outcome;
  }

  remove(node: NodeKey, name: string): Promise<Outcome<unknown>> {
    return attempt(() =>
      this.#fleet.bridge.call(node, "workspace.label.delete", { name }),
    );
  }

  /** Start loops for newly connected machines; mark the others offline. */
  #follow(): void {
    const statuses = this.#fleet.store.get().statuses;
    for (const [node, status] of entriesOf(statuses)) {
      const connected = status.state === "connected";
      if (connected && !this.#running.has(node)) {
        const run = Symbol(node);
        this.#running.set(node, run);
        void this.#registryLoop(node, run);
        void this.#catalogLoop(node, run);
      } else if (!connected && this.#running.has(node)) {
        this.#running.delete(node);
        this.#setCatalog(node, (held) => ({ ...held, state: "offline" }));
      }
    }
  }

  #alive(node: NodeKey, run: symbol): boolean {
    return this.#running.get(node) === run;
  }

  async #registryLoop(node: NodeKey, run: symbol): Promise<void> {
    let revision: number | null = null;
    while (this.#alive(node, run)) {
      const outcome = await attempt(() =>
        this.#fleet.bridge.call(node, "workspace.registry", {
          changed_from: revision,
        }),
      );
      if (!this.#alive(node, run)) {
        return;
      }
      if (outcome.ok) {
        revision = outcome.value.revision;
        const snapshot = outcome.value;
        this.store.update((state) =>
          sameRegistry(state.registries[node], snapshot)
            ? state
            : {
                ...state,
                registries: { ...state.registries, [node]: snapshot },
              },
        );
      } else if (unsupported(outcome)) {
        return;
      } else {
        revision = null;
        await pause(RETRY_MS);
      }
    }
  }

  async #catalogLoop(node: NodeKey, run: symbol): Promise<void> {
    const replica = new CatalogReplica();
    this.#setCatalog(node, (held) => ({ ...held, state: "loading" }));
    while (this.#alive(node, run)) {
      const head = replica.head;
      const outcome = await attempt(() =>
        this.#fleet.bridge.call(node, "workspace.label.list", {
          cursor: head,
          read: head === null ? "now" : "observe",
        }),
      );
      if (!this.#alive(node, run)) {
        return;
      }
      if (outcome.ok) {
        if (!replica.apply(outcome.value)) {
          replica.reset();
          continue;
        }
        const labels = replica.labels();
        this.#setCatalog(node, () => ({
          state: "online",
          labels,
          error: null,
        }));
      } else if (unsupported(outcome)) {
        this.#setCatalog(node, () => ({
          state: "unsupported",
          labels: [],
          error: null,
        }));
        return;
      } else {
        const error = outcome.problem.message;
        replica.reset();
        this.#setCatalog(node, (held) => ({ ...held, error }));
        await pause(RETRY_MS);
      }
    }
  }

  #setCatalog(
    node: NodeKey,
    change: (held: MachineLabels) => MachineLabels,
  ): void {
    this.store.update((state) => {
      const held = state.catalogs[node] ?? {
        state: "loading",
        labels: [],
        error: null,
      };
      return {
        ...state,
        catalogs: { ...state.catalogs, [node]: change(held) },
      };
    });
  }
}

const HUBS = new WeakMap<Fleet, WorkspaceLabels>();

/** The one hub of `fleet`, made on first use. */
export function labelsOf(fleet: Fleet): WorkspaceLabels {
  let hub = HUBS.get(fleet);
  if (hub === undefined) {
    hub = new WorkspaceLabels(fleet);
    HUBS.set(fleet, hub);
  }
  return hub;
}

/** Whether the label manager is open. It is opened from the palette and
 * the view menu and drawn once by the app's root, so its state lives
 * here (as the add-machine dialog's does). */
export const labelManager = new Store<boolean>(false);
