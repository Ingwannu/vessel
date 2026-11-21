// The route owns this controller's lifetime, so Schedule RPCs go quiet
// elsewhere. Each connection gets its own request marker because a slow
// reply from a previous connection must never replace the current list.
import type { Bridge } from "../../bridge/client";
import type { BookNode, NodeKey, NodeStatus } from "../../wire/bridge";
import type { ScheduleId, ScheduleInfo } from "../../wire/proto";
import type { FleetState } from "../fleet";
import { Store } from "../store";
import { type ScheduleAction, rowActions } from "./rows";

type Read =
  | { kind: "loading" }
  | { kind: "failed"; reason: string }
  | { kind: "ready"; schedules: readonly ScheduleInfo[] };

interface MachineRead {
  generation: symbol;
  connection: NodeStatus | undefined;
  read: Read;
  pending: ReadonlyMap<ScheduleId, ScheduleAction>;
}

export interface ScheduleReads {
  book: readonly BookNode[] | null;
  bookError: string | null;
  machines: ReadonlyMap<NodeKey, MachineRead>;
}

export class MachineSchedules {
  readonly store = new Store<ScheduleReads>({
    book: null,
    bookError: null,
    machines: new Map(),
  });

  readonly #bridge: Pick<Bridge, "call">;
  readonly #requests = new Map<NodeKey, symbol>();

  constructor(bridge: Pick<Bridge, "call">) {
    this.#bridge = bridge;
  }

  sync({ book, bookError, statuses, link }: FleetState): void {
    const old = this.store.get();
    const machines = new Map<NodeKey, MachineRead>();
    const read: NodeKey[] = [];
    let changed = book !== old.book || bookError !== old.bookError;
    for (const node of book ?? []) {
      const connection = link.kind === "open" ? statuses[node.key] : undefined;
      const previous = old.machines.get(node.key);
      if (previous !== undefined && previous.connection === connection) {
        machines.set(node.key, previous);
        continue;
      }
      changed = true;
      this.#requests.delete(node.key);
      machines.set(node.key, {
        generation: Symbol(),
        connection,
        read: initialRead(connection),
        pending: new Map(),
      });
      if (supportsSchedules(connection)) read.push(node.key);
    }
    for (const key of old.machines.keys()) {
      if (!machines.has(key)) this.#requests.delete(key);
    }
    if (changed) this.store.set({ book, bookError, machines });
    for (const node of read) this.refresh(node);
  }

  stop(): void {
    this.#requests.clear();
    // An effect can be mounted twice by React StrictMode.
    this.store.set({ book: null, bookError: null, machines: new Map() });
  }

  retry(): void {
    for (const [node, machine] of this.store.get().machines) {
      if (
        machine.read.kind === "failed" &&
        supportsSchedules(machine.connection)
      ) {
        this.#set(node, { ...machine, read: { kind: "loading" } });
        this.refresh(node);
      }
    }
  }

  refresh(node: NodeKey): void {
    const machine = this.store.get().machines.get(node);
    if (machine === undefined || !supportsSchedules(machine.connection)) return;
    const request = Symbol();
    this.#requests.set(node, request);
    const settle = (read: Read) => {
      if (this.#requests.get(node) !== request) return;
      this.#requests.delete(node);
      const latest = this.store.get().machines.get(node);
      if (latest !== undefined) this.#set(node, { ...latest, read });
    };
    void this.#bridge.call(node, "schedule.list", {}).then(
      ({ schedules }) => settle({ kind: "ready", schedules }),
      () => settle({ kind: "failed", reason: "Could not load schedules" }),
    );
  }

  async act(
    node: NodeKey,
    id: ScheduleId,
    action: ScheduleAction,
  ): Promise<void> {
    const machine = this.store.get().machines.get(node);
    if (machine?.read.kind !== "ready" || machine.pending.has(id)) return;
    const row = machine.read.schedules.find((info) => info.id === id);
    if (row === undefined || !rowActions(row).includes(action)) return;

    const pending = new Map(machine.pending).set(id, action);
    this.#set(node, { ...machine, pending });
    let replace: ScheduleInfo | undefined;
    let deleted = false;
    try {
      switch (action) {
        case "pause":
          replace = await this.#bridge.call(node, "schedule.pause", {
            schedule: id,
          });
          break;
        case "resume":
          replace = await this.#bridge.call(node, "schedule.resume", {
            schedule: id,
          });
          break;
        case "run":
          await this.#bridge.call(node, "schedule.run", { schedule: id });
          break;
        case "delete":
          await this.#bridge.call(node, "schedule.delete", { schedule: id });
          deleted = true;
          break;
      }
    } finally {
      this.#settleAction(node, id, machine.generation, replace, deleted);
    }
  }

  #settleAction(
    node: NodeKey,
    id: ScheduleId,
    generation: symbol,
    replace: ScheduleInfo | undefined,
    deleted: boolean,
  ): void {
    const latest = this.store.get().machines.get(node);
    if (latest === undefined || latest.generation !== generation) return;
    // Keep the server snapshot under the pending overlay. A refusal removes
    // only its overlay, without reverting another row's successful action.
    const pending = new Map(latest.pending);
    pending.delete(id);
    const read = actionReply(latest.read, id, replace, deleted);
    this.#set(node, { ...latest, pending, read });
    this.refresh(node);
  }

  #set(node: NodeKey, machine: MachineRead): void {
    this.store.update((state) => ({
      ...state,
      machines: new Map(state.machines).set(node, machine),
    }));
  }
}

function initialRead(connection: NodeStatus | undefined): Read {
  switch (connection?.state) {
    case "failed":
      return { kind: "failed", reason: "Could not load schedules" };
    case "connected":
      return supportsSchedules(connection)
        ? { kind: "loading" }
        : {
            kind: "failed",
            reason: "Schedules are unavailable on this machine",
          };
    case undefined:
    case "idle":
    case "connecting":
    case "reconnecting":
      return { kind: "loading" };
  }
}

export function supportsSchedules(connection: NodeStatus | undefined): boolean {
  return connection?.state === "connected" && connection.protocol >= 11;
}

function actionReply(
  read: Read,
  id: ScheduleId,
  replace: ScheduleInfo | undefined,
  deleted: boolean,
): Read {
  if (read.kind !== "ready") return read;
  if (deleted) {
    return {
      kind: "ready",
      schedules: read.schedules.filter((info) => info.id !== id),
    };
  }
  return replace === undefined
    ? read
    : {
        kind: "ready",
        schedules: read.schedules.map((info) =>
          info.id === id ? replace : info,
        ),
      };
}
