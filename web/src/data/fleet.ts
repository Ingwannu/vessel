// The fleet as the page knows it: the book's nodes, each node's link
// state (pushed by the bridge), and what each connected node reports
// (harnesses, profiles, model routes, sessions, and the agents added on
// it, read only where one shows: `loadAgents`).
//
// Sessions have no list subscription in the protocol, so each connected
// node's list is read again every few seconds while the page is visible,
// and while it is hidden too when notices are on (Settings ›
// Notifications: a hidden page is the one they are for; the browser
// slows a hidden tab's timers, to about once a minute after five
// minutes). Rows patch in place because unchanged sessions keep their
// objects, and a list with no change leaves the state as it was, so
// nothing is drawn again (milestone 11, docs/PERFORMANCE.md; the cards'
// clock is clock.ts).

import { Bridge, CallFailed, type LinkState } from "../bridge/client";
import type { BookNode, NodeKey, NodeStatus } from "../wire/bridge";
import type {
  AgentInfo,
  Harness,
  ModelRoute,
  ProfileInfo,
  SessionInfo,
} from "../wire/proto";
import { wantsNotices } from "./alerts";
import { Drafts } from "./drafts";
import { prefs } from "./prefs";
import { keep, withoutSession, withSession } from "./fleet-sessions";
import { entriesOf } from "./records";
import { Store } from "./store";

const SESSIONS_EVERY_MS = 2000;

export interface FleetState {
  link: LinkState;
  /** The nodes to list; `null` until the bridge has answered. */
  book: readonly BookNode[] | null;
  bookError: string | null;
  statuses: Readonly<Record<NodeKey, NodeStatus>>;
  harnesses: Readonly<Record<NodeKey, readonly Harness[]>>;
  profiles: Readonly<Record<NodeKey, readonly ProfileInfo[]>>;
  /** The model routes each node offers (`model.list`, protocol 7);
   * `null` for a node older than model routing. */
  models: Readonly<Record<NodeKey, readonly ModelRoute[] | null>>;
  sessions: Readonly<Record<NodeKey, readonly SessionInfo[]>>;
  /** Every agent a node lists with what it knows of each (`agent.list`,
   * protocol 11): read once a node shows an added agent, or Settings ›
   * Agents opens; `[]` from an older node. */
  agents: Readonly<Record<NodeKey, readonly AgentInfo[]>>;
  /** The last error reading a node, by node. */
  errors: Readonly<Record<NodeKey, string>>;
  /** The node the operator last chose (DESIGN.md §2.5). */
  selected: NodeKey | null;
}

const INITIAL: FleetState = {
  link: { kind: "connecting", attempt: 0 },
  book: null,
  bookError: null,
  statuses: {},
  harnesses: {},
  profiles: {},
  models: {},
  sessions: {},
  agents: {},
  errors: {},
  selected: null,
};

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export class Fleet {
  readonly bridge: Bridge;
  readonly store = new Store<FleetState>(INITIAL);
  /** The composers' drafts, kept by the bridge (data/drafts.ts). */
  readonly drafts: Drafts;
  #timer: ReturnType<typeof setInterval> | null = null;

  constructor(bridge: Bridge) {
    this.bridge = bridge;
    this.drafts = new Drafts(this);
    bridge.onLink((link) => {
      this.store.update((state) => ({ ...state, link }));
      if (link.kind === "open") {
        void this.loadBook();
        void this.drafts.readAll();
      }
    });
    bridge.onNode((node, status) => {
      const before = this.store.get().statuses[node];
      this.store.update((state) => ({
        ...state,
        statuses: { ...state.statuses, [node]: status },
      }));
      if (status.state === "connected" && before?.state !== "connected") {
        void this.refresh(node);
      }
    });
  }

  start(): void {
    this.bridge.start();
    this.#timer = setInterval(() => {
      if (
        document.visibilityState === "visible" ||
        wantsNotices(prefs.get().values.notifications)
      ) {
        this.#pollSessions();
      }
    }, SESSIONS_EVERY_MS);
  }

  stop(): void {
    if (this.#timer !== null) {
      clearInterval(this.#timer);
    }
    this.bridge.stop();
  }

  select(node: NodeKey | null): void {
    this.store.update((state) =>
      state.selected === node ? state : { ...state, selected: node },
    );
  }

  /** The book's nodes, then a first read of each (which connects it). */
  async loadBook(): Promise<void> {
    try {
      const result = await this.bridge.op({ op: "nodes" });
      if (result.op !== "nodes") {
        return;
      }
      this.store.update((state) => ({
        ...state,
        book: result.nodes,
        bookError: null,
      }));
      await Promise.all(result.nodes.map((node) => this.refresh(node.key)));
    } catch (error) {
      this.store.update((state) => ({ ...state, bookError: message(error) }));
    }
  }

  /** The book as an edit from the page left it (`forget`, `rename` and
   * `default` answer with it, so no second round trip): what left the
   * book is dropped from every map, so nothing keeps listing or polling
   * a forgotten machine, and a name new to it (a renamed machine) is
   * read, which connects it. */
  async takeBook(nodes: readonly BookNode[]): Promise<void> {
    const keys = new Set(nodes.map((node) => node.key));
    const before = new Set((this.store.get().book ?? []).map((n) => n.key));
    const kept = <V>(map: Readonly<Record<NodeKey, V>>) =>
      Object.fromEntries(entriesOf(map).filter(([key]) => keys.has(key)));
    this.store.update((state) => ({
      ...state,
      book: nodes,
      bookError: null,
      statuses: kept(state.statuses),
      harnesses: kept(state.harnesses),
      profiles: kept(state.profiles),
      models: kept(state.models),
      sessions: kept(state.sessions),
      agents: kept(state.agents),
      errors: kept(state.errors),
      selected:
        state.selected !== null && keys.has(state.selected)
          ? state.selected
          : null,
    }));
    await Promise.all(
      nodes
        .filter((node) => !before.has(node.key))
        .map((node) => this.refresh(node.key)),
    );
  }

  /** Read what `node` reports. */
  async refresh(node: NodeKey): Promise<void> {
    try {
      const [harnesses, profiles, models, sessions] = await Promise.all([
        this.bridge.call(node, "harness.list", null),
        this.bridge.call(node, "profile.list", {}),
        this.#models(node),
        this.bridge.call(node, "session.list", null),
      ]);
      this.store.update((state) => {
        const errors = Object.fromEntries(
          Object.entries(state.errors).filter(([key]) => key !== node),
        );
        return {
          ...state,
          harnesses: { ...state.harnesses, [node]: harnesses },
          profiles: { ...state.profiles, [node]: profiles },
          models: { ...state.models, [node]: models },
          sessions: {
            ...state.sessions,
            [node]: keep(state.sessions[node], sessions),
          },
          errors,
        };
      });
      if (
        harnesses.some((harness) => typeof harness.kind !== "string") ||
        sessions.some((session) => typeof session.spec.harness !== "string")
      ) {
        await this.loadAgents(node);
      }
    } catch (error) {
      this.store.update((state) => ({
        ...state,
        errors: { ...state.errors, [node]: message(error) },
      }));
    }
  }

  /** Read the agents `node` lists, for an added agent's label and
   * Settings › Agents. An older node has none to list; a failure keeps
   * what was read before (the name stands in for a label). */
  async loadAgents(node: NodeKey): Promise<void> {
    let agents: readonly AgentInfo[];
    try {
      agents = await this.bridge.call(node, "agent.list", {});
    } catch (error) {
      if (
        !(error instanceof CallFailed) ||
        error.failure.kind !== "not_in_protocol"
      ) {
        return;
      }
      agents = [];
    }
    this.store.update((state) => ({
      ...state,
      agents: { ...state.agents, [node]: agents },
    }));
  }

  /** `node`'s routes; `null` when it predates them, so the rest of what
   * it reports still shows. */
  async #models(node: NodeKey): Promise<readonly ModelRoute[] | null> {
    try {
      return await this.bridge.call(node, "model.list", null);
    } catch (error) {
      if (
        error instanceof CallFailed &&
        error.failure.kind === "not_in_protocol"
      ) {
        return null;
      }
      throw error;
    }
  }

  async refreshSessions(node: NodeKey): Promise<void> {
    try {
      const sessions = await this.bridge.call(node, "session.list", null);
      this.store.update((state) => {
        const before = state.sessions[node];
        const after = keep(before, sessions);
        return after === before
          ? state
          : { ...state, sessions: { ...state.sessions, [node]: after } };
      });
    } catch {
      // The node's state says why; the list stays as it was.
    }
  }

  /** Put a session the node just answered with (a mark, a start again)
   * into the machine's list at once, rather than at the next poll, so
   * every list shows it the moment the node says. */
  putSession(node: NodeKey, session: SessionInfo): void {
    this.store.update((state) => withSession(state, node, session));
  }

  /** Take a deleted session out of the machine's list at once. */
  dropSession(node: NodeKey, session: SessionInfo["id"]): void {
    this.store.update((state) => withoutSession(state, node, session));
  }

  #pollSessions(): void {
    for (const [node, status] of entriesOf(this.store.get().statuses)) {
      if (status.state === "connected") {
        void this.refreshSessions(node);
      }
    }
  }
}
