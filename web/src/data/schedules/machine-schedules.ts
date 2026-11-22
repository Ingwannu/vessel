// The route owns this controller's lifetime, so Schedule RPCs go quiet
// elsewhere. Each connection gets its own request marker because a slow
// reply from a previous connection must never replace the current list.
import { type Bridge, CallFailed } from "../../bridge/client";
import type { BookNode, NodeKey, NodeStatus } from "../../wire/bridge";
import type {
  ScheduleId,
  ScheduleInfo,
  SchedulePreview,
} from "../../wire/proto";
import type { FleetState } from "../fleet";
import { Store } from "../store";
import {
  confirmsHeartbeat,
  type Cron,
  editableHeartbeat,
  heartbeatRequest,
  HeartbeatSaveUnconfirmed,
  supportsHeartbeatEdit,
} from "./heartbeat";
import {
  actionReply,
  initialRead,
  type ScheduleRead,
  supportsSchedules,
} from "./machine-read";
import {
  type ScheduleAction,
  type SchedulePending,
  type ScheduleRow,
  rowActions,
} from "./rows";

export { supportsSchedules } from "./machine-read";

interface MachineRead {
  generation: symbol;
  connection: NodeStatus | undefined;
  read: ScheduleRead;
  pending: ReadonlyMap<ScheduleId, SchedulePending>;
  // Undefined until a relevant list starts this connection's grant read.
  heartbeatGrant?: boolean;
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
    const settle = (read: ScheduleRead) => {
      if (this.#requests.get(node) !== request) return;
      this.#requests.delete(node);
      const latest = this.store.get().machines.get(node);
      if (latest === undefined) return;
      this.#set(node, { ...latest, read });
      if (read.kind === "ready" && read.schedules.some(editableHeartbeat))
        this.#readHeartbeatGrant(node);
    };
    void this.#bridge.call(node, "schedule.list", {}).then(
      ({ schedules }) => {
        settle({ kind: "ready", schedules });
      },
      () => {
        settle({ kind: "failed", reason: "Could not load schedules" });
      },
    );
  }

  #readHeartbeatGrant(node: NodeKey): void {
    const machine = this.store.get().machines.get(node);
    if (
      machine === undefined ||
      machine.heartbeatGrant !== undefined ||
      !supportsHeartbeatEdit(machine.connection)
    )
      return;
    this.#set(node, { ...machine, heartbeatGrant: false });
    void this.#bridge.call(node, "device.self", {}).then(
      ({ scopes }) => {
        const latest = this.store.get().machines.get(node);
        if (
          latest?.generation === machine.generation &&
          scopes.includes("operate")
        )
          this.#set(node, { ...latest, heartbeatGrant: true });
      },
      () => {
        // A grant which cannot be read cannot offer a write.
      },
    );
  }

  heartbeatAvailable(row: ScheduleRow): boolean {
    const machine = this.store.get().machines.get(row.node);
    return (
      machine !== undefined &&
      machine.generation === row.heartbeatRoute &&
      machine.heartbeatGrant === true
    );
  }

  async saveHeartbeat(
    row: ScheduleRow,
    draft: Cron,
    preview: SchedulePreview,
  ): Promise<void> {
    const { node, info } = row;
    const machine = this.store.get().machines.get(node);
    if (
      machine === undefined ||
      machine.generation !== row.heartbeatRoute ||
      machine.heartbeatGrant !== true
    )
      throw new Error(
        "The original heartbeat connection is unavailable. Reopen it to edit.",
      );
    if (machine.pending.has(info.id))
      throw new Error("Another change to this heartbeat is pending.");
    const request = heartbeatRequest(info, draft, preview);
    if (request === null)
      throw new Error("Choose a new, previewed cron cadence.");

    const pending = new Map(machine.pending).set(info.id, "change_cadence");
    this.#set(node, { ...machine, pending });
    let replace: ScheduleInfo | undefined;
    try {
      const received = await this.#bridge.call(
        node,
        "schedule.update_cadence",
        request,
      );
      if (
        !this.heartbeatAvailable(row) ||
        !confirmsHeartbeat(info, request, received)
      )
        throw new HeartbeatSaveUnconfirmed();
      const read = this.store.get().machines.get(node)?.read;
      const current =
        read?.kind === "ready"
          ? read.schedules.find((item) => item.id === info.id)
          : undefined;
      // A newer observation or a deletion during the save takes precedence.
      if (current?.revision === info.revision) replace = received;
    } catch (error) {
      if (
        error instanceof CallFailed &&
        (error.failure.kind === "disconnected" ||
          error.failure.kind === "failed")
      )
        throw new HeartbeatSaveUnconfirmed();
      throw error;
    } finally {
      this.#settleAction(node, info.id, machine.generation, replace, false);
    }
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
    if (latest?.generation !== generation) return;
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
