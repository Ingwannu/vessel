// A call to a node that has no reply, and how it is said: the bridge's
// typed reason kept (`Failure`), so a screen decides what to do next by
// switching on it, never by reading words.

import type { Failure } from "../wire/bridge";

/** A call that has no reply, with the bridge's reason. */
export class CallFailed extends Error {
  readonly failure: Failure;

  constructor(failure: Failure) {
    super(describeFailure(failure));
    this.name = "CallFailed";
    this.failure = failure;
  }
}

/** A failure as one line for the operator. */
export function describeFailure(failure: Failure): string {
  switch (failure.kind) {
    case "remote":
      return failure.error.message;
    case "not_in_protocol":
      return `${failure.method} needs protocol ${failure.needs}; the node speaks ${failure.negotiated}`;
    case "disconnected":
      return "The connection closed before the node answered; it may or may not have run.";
    case "unreachable":
    case "refused":
    case "failed":
      return failure.message;
  }
}
