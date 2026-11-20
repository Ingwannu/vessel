// Track the cursor for both bridge stream kinds, including reconnect.

import type {
  ClientFrame,
  NodeKey,
  StreamId,
  StreamState,
} from "../wire/bridge";
import type { EventRecord, SessionId, TerminalId } from "../wire/proto";

export interface StreamHandlers {
  record: (record: EventRecord) => void;
  state: (state: StreamState) => void;
}

export type FollowTarget =
  | { type: "follow"; session: SessionId }
  | { type: "follow_terminal"; terminal: TerminalId };

interface Followed {
  node: NodeKey;
  target: FollowTarget;
  next: number;
  handlers: StreamHandlers;
}

function frame(stream: StreamId, followed: Followed): ClientFrame {
  return {
    ...followed.target,
    stream,
    node: followed.node,
    from: followed.next,
  };
}

export class Followers {
  readonly #streams = new Map<StreamId, Followed>();

  open(stream: StreamId, followed: Followed): ClientFrame {
    this.#streams.set(stream, followed);
    return frame(stream, followed);
  }

  close(stream: StreamId): boolean {
    return this.#streams.delete(stream);
  }

  rejoin(send: (message: ClientFrame) => void): void {
    for (const [stream, followed] of this.#streams) {
      send(frame(stream, followed));
    }
  }

  receive(stream: StreamId, record: EventRecord): void {
    const followed = this.#streams.get(stream);
    if (followed === undefined || record.seq < followed.next) {
      return;
    }
    followed.next = record.seq + 1;
    followed.handlers.record(record);
  }

  state(stream: StreamId, state: StreamState): void {
    const followed = this.#streams.get(stream);
    if (followed === undefined) {
      return;
    }
    if (state.state === "missing") {
      followed.next = state.resumed_at;
    }
    if (state.state === "ended" || state.state === "failed") {
      this.close(stream);
    }
    followed.handlers.state(state);
  }
}
