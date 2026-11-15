// The page's side of the bridge (ADR 0017): typed calls, ops, followed
// sessions and node states, over the one socket `socket.ts` keeps open.
//
// [Decision Log]
// - Purpose: every screen talks to nodes through this one socket. It
//   must survive the socket dropping without losing or repeating a
//   session's records, and never repeat a call that may have run.
// - Choice: calls are promises keyed by id; when the socket drops, every
//   call in flight fails as `disconnected` (the node may have run it, so
//   nothing retries it). Streams remember the next record they need and
//   are followed again from it after each welcome; the bridge's own
//   streams are gap-free, so the stores just continue. Node states are
//   kept as the bridge last pushed them, and pushed again in full after a
//   welcome.
// - The socket's lifecycle (dialling, the proof, redialling, refusals) is
//   `socket.ts`; a call's failure and its words are `failure.ts`.

import { CallFailed } from "./failure";
import * as frames from "./frames";
import { callIdOf, streamIdOf } from "./ids";
import { BridgeSocket, type LinkState, socketUrl } from "./socket";
import type { Credential } from "./token";
import type {
  CallId,
  NodeKey,
  NodeStatus,
  Op,
  OpResult,
  PreferencesState,
  ServerFrame,
  StreamId,
  StreamState,
} from "../wire/bridge";
import type { EventRecord, NodeId, Reply, SessionId } from "../wire/proto";

export { CallFailed, describeFailure } from "./failure";
export type { LinkState } from "./socket";

/** What a followed session's stream reports. */
export interface StreamHandlers {
  record: (record: EventRecord) => void;
  state: (state: StreamState) => void;
}

interface Pending {
  resolve: (reply: Reply) => void;
  reject: (error: CallFailed) => void;
}

interface PendingOp {
  resolve: (result: OpResult) => void;
  reject: (error: Error) => void;
}

interface Followed {
  node: NodeKey;
  session: SessionId;
  /** The next record wanted. */
  next: number;
  handlers: StreamHandlers;
}

export class Bridge {
  readonly #socket: BridgeSocket;
  #nextId = 1;
  readonly #calls = new Map<CallId, Pending>();
  readonly #ops = new Map<CallId, PendingOp>();
  readonly #streams = new Map<StreamId, Followed>();
  readonly #statuses = new Map<NodeKey, NodeStatus>();
  readonly #nodeListeners = new Set<
    (node: NodeKey, status: NodeStatus) => void
  >();
  readonly #linkListeners = new Set<(link: LinkState) => void>();
  readonly #preferenceListeners = new Set<(state: PreferencesState) => void>();

  constructor(token: Credential, url: string = socketUrl()) {
    this.#socket = new BridgeSocket(token, url, {
      frame: (frame) => {
        this.#receive(frame);
      },
      link: (link) => {
        this.#linked(link);
      },
      lost: () => {
        this.#lost();
      },
    });
  }

  get link(): LinkState {
    return this.#socket.link;
  }

  /** Open the socket (and keep it open). */
  start(): void {
    this.#socket.start();
  }

  /** Dial again now (after `disconnected`), with the credential kept. */
  retry(): void {
    this.#socket.retry();
  }

  /** Close for good. */
  stop(): void {
    this.#socket.stop();
  }

  /** Call `method` on `node`. Rejects with `CallFailed`. */
  async call<M extends frames.Method>(
    node: NodeKey,
    method: M,
    params: frames.ParamsOf<M>,
    expectedNode?: NodeId,
  ): Promise<frames.ResultOf<M>> {
    const id = callIdOf(this.#id());
    const reply = await new Promise<Reply>((resolve, reject) => {
      this.#calls.set(id, { resolve, reject });
      if (
        !this.#socket.send(
          frames.frameOf(id, node, method, params, expectedNode),
        )
      ) {
        this.#calls.delete(id);
        reject(
          new CallFailed({
            kind: "unreachable",
            message: "Not connected to vessel ui.",
          }),
        );
      }
    });
    if (!frames.answers(reply, method)) {
      throw new CallFailed({
        kind: "failed",
        message: `The bridge answered ${reply.method} to ${method}.`,
      });
    }
    return reply.result;
  }

  /** Ask the bridge itself. Rejects with an `OpFailed` keeping its reason. */
  op(op: Op): Promise<OpResult> {
    const id = callIdOf(this.#id());
    return new Promise<OpResult>((resolve, reject) => {
      this.#ops.set(id, { resolve, reject });
      if (!this.#socket.send({ type: "op", id, op })) {
        this.#ops.delete(id);
        reject(frames.opLost("Not connected to vessel ui."));
      }
    });
  }

  /** Follow `session` from record `from`; the returned function stops. */
  follow(
    node: NodeKey,
    session: SessionId,
    from: number,
    handlers: StreamHandlers,
  ): () => void {
    const stream = streamIdOf(this.#id());
    this.#streams.set(stream, { node, session, next: from, handlers });
    this.#socket.send({ type: "follow", stream, node, session, from });
    return () => {
      if (this.#streams.delete(stream)) {
        this.#socket.send({ type: "unfollow", stream });
      }
    };
  }

  /** Every node's last known state. */
  statuses(): ReadonlyMap<NodeKey, NodeStatus> {
    return this.#statuses;
  }

  onNode(listener: (node: NodeKey, status: NodeStatus) => void): () => void {
    this.#nodeListeners.add(listener);
    return () => this.#nodeListeners.delete(listener);
  }

  onLink(listener: (link: LinkState) => void): () => void {
    this.#linkListeners.add(listener);
    return () => this.#linkListeners.delete(listener);
  }

  /** Every preferences state the bridge pushes (ADR 0028): once after
   * each welcome, then after every change from any tab. */
  onPreferences(listener: (state: PreferencesState) => void): () => void {
    this.#preferenceListeners.add(listener);
    return () => this.#preferenceListeners.delete(listener);
  }

  #id(): number {
    const id = this.#nextId;
    this.#nextId += 1;
    return id;
  }

  #linked(link: LinkState): void {
    for (const listener of this.#linkListeners) {
      listener(link);
    }
    if (link.kind !== "open") {
      return;
    }
    // Every stream continues where it was.
    for (const [stream, followed] of this.#streams) {
      this.#socket.send({
        type: "follow",
        stream,
        node: followed.node,
        session: followed.session,
        from: followed.next,
      });
    }
  }

  /** The socket went: calls in flight fail, never retried. */
  #lost(): void {
    for (const pending of this.#calls.values()) {
      pending.reject(new CallFailed({ kind: "disconnected" }));
    }
    this.#calls.clear();
    for (const pending of this.#ops.values()) {
      pending.reject(frames.opLost("The connection to vessel ui closed."));
    }
    this.#ops.clear();
  }

  #receive(frame: ServerFrame): void {
    switch (frame.type) {
      case "challenge":
      case "welcome":
        return;
      case "reply": {
        const pending = this.#calls.get(frame.id);
        this.#calls.delete(frame.id);
        if (pending === undefined) {
          return;
        }
        if ("ok" in frame.outcome) {
          pending.resolve(frame.outcome.ok);
        } else {
          pending.reject(new CallFailed(frame.outcome.err));
        }
        return;
      }
      case "op_reply": {
        const pending = this.#ops.get(frame.id);
        this.#ops.delete(frame.id);
        if (pending === undefined) {
          return;
        }
        if ("ok" in frame.outcome) {
          pending.resolve(frame.outcome.ok);
        } else {
          pending.reject(frames.opFailed(frame.outcome.err));
        }
        return;
      }
      case "node":
        this.#statuses.set(frame.node, frame.status);
        for (const listener of this.#nodeListeners) {
          listener(frame.node, frame.status);
        }
        return;
      case "event": {
        const followed = this.#streams.get(frame.stream);
        if (followed === undefined || frame.record.seq < followed.next) {
          return;
        }
        followed.next = frame.record.seq + 1;
        followed.handlers.record(frame.record);
        return;
      }
      case "stream": {
        const followed = this.#streams.get(frame.stream);
        if (followed === undefined) {
          return;
        }
        if (frame.state.state === "missing") {
          followed.next = frame.state.resumed_at;
        }
        if (frame.state.state === "ended" || frame.state.state === "failed") {
          this.#streams.delete(frame.stream);
        }
        followed.handlers.state(frame.state);
        return;
      }
      case "preferences":
        for (const listener of this.#preferenceListeners) {
          listener(frame.preferences);
        }
        return;
      case "unreadable":
        console.warn(`vessel ui could not read a frame: ${frame.message}`);
        return;
    }
  }
}
