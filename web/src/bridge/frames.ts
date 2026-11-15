// What crosses the socket, narrowed instead of cast (the plan's "no cast
// at a call site"): a frame from `JSON.parse` is `unknown` until its
// discriminant is checked; a call is built by a function generic over the
// method map, so its parameters are the method's; a reply is narrowed by
// its method before its result is read.
//
// The checks are of shape, not of every field: the bridge is this
// machine's own `vessel ui`, authenticated before any frame is read
// (ADR 0017), and its frames are generated from the same Rust types as
// these. What they catch is a frame of another kind or version.

import type { CallId, NodeKey, OpFailure, ServerFrame } from "../wire/bridge";
import type { Call, EventRecord, NodeId, Reply } from "../wire/proto";

/** The kinds of frame the bridge sends. */
const SERVER_FRAMES: ReadonlySet<string> = new Set([
  "challenge",
  "welcome",
  "reply",
  "op_reply",
  "node",
  "event",
  "stream",
  "preferences",
  "unreadable",
] satisfies ServerFrame["type"][]);

/** Whether `value` is one of a session's event records. */
export function isEventRecord(value: unknown): value is EventRecord {
  return (
    typeof value === "object" &&
    value !== null &&
    "seq" in value &&
    typeof value.seq === "number" &&
    "at" in value &&
    typeof value.at === "string" &&
    "event" in value &&
    typeof value.event === "object" &&
    value.event !== null &&
    "type" in value.event &&
    typeof value.event.type === "string"
  );
}

/** Whether `value`, parsed from the socket, is a frame of the bridge's. */
export function isServerFrame(value: unknown): value is ServerFrame {
  if (
    typeof value !== "object" ||
    value === null ||
    !("type" in value) ||
    typeof value.type !== "string" ||
    !SERVER_FRAMES.has(value.type)
  ) {
    return false;
  }
  return (
    value.type !== "event" || ("record" in value && isEventRecord(value.record))
  );
}

/** The frame in a socket message's data; throws on anything else. */
export function parse(data: unknown): ServerFrame {
  if (typeof data !== "string") {
    throw new Error("Not a frame");
  }
  const frame: unknown = JSON.parse(data);
  if (!isServerFrame(frame)) {
    throw new Error("Not a frame");
  }
  return frame;
}

/** Each method's parameters and result, keyed by its wire name. */
type Params = { [C in Call as C["method"]]: C["params"] };
type Results = { [R in Reply as R["method"]]: R["result"] };

/** A method's wire name. */
export type Method = keyof Params;

/** The call of method `M`: for a known method, exactly that member of
 * `Call` (frames.test.ts checks it). Built through a mapped type so that
 * `callOf` needs no assertion.
 *
 * Why the frame that carries it is `CallFrame` and not the generated
 * `ClientFrame`: TypeScript cannot relate a call built for a generic `M`
 * to the `Call` union (a known limit of correlated unions; the compiler
 * reduces the target to an intersection of every member). The two shapes
 * are the same for every method, and the bridge only serialises it. */
export type CallOf<M extends Method> = {
  [K in M]: { method: K; params: Params[K] };
}[M];

/** The frame that sends a call: `ClientFrame`'s `call` member, with the
 * call typed by its method. */
export interface CallFrame<M extends Method> {
  type: "call";
  id: CallId;
  node: NodeKey;
  call: CallOf<M>;
  /** A captured authenticated recipient; mismatches refuse before dispatch. */
  expected_node?: NodeId;
}

/** The reply to method `M`, the same way. */
export type ReplyOf<M extends Method> = {
  [K in Method]: { method: K; result: Results[K] };
}[M];

/** What `method` answers. */
export type ResultOf<M extends Method> = Results[M];

/** What `method` takes. */
export type ParamsOf<M extends Method> = Params[M];

/** The call of `method` with `params`. */
export function callOf<M extends Method>(
  method: M,
  params: ParamsOf<M>,
): CallOf<M> {
  return { method, params };
}

/** A prepared recipient constraint is omitted for ordinary legacy calls. */
export function frameOf<M extends Method>(
  id: CallId,
  node: NodeKey,
  method: M,
  params: ParamsOf<M>,
  expectedNode?: NodeId,
): CallFrame<M> {
  return {
    type: "call",
    id,
    node,
    call: callOf(method, params),
    ...(expectedNode === undefined ? {} : { expected_node: expectedNode }),
  };
}

/** Whether `reply` answers `method`. */
export function answers<M extends Method>(
  reply: Reply,
  method: M,
): reply is Reply & ReplyOf<M> {
  return reply.method === method;
}

/** Why an op has no result: the bridge's own reason, or `disconnected`
 * when the socket was not open or closed before the answer. */
export type OpReason = OpFailure | "disconnected";

/** An op that has no result, with its reason kept, so the page can say
 * what to do next without reading the message (track W2: "busy" is to
 * wait and try again, "failed" is the book's or the link's own words). */
export class OpFailed extends Error {
  readonly reason: OpReason;

  constructor(reason: OpReason, message: string) {
    super(message);
    this.name = "OpFailed";
    this.reason = reason;
  }
}

/** An op's failure as the bridge answered it. */
export function opFailed(err: {
  reason: OpFailure;
  message: string;
}): OpFailed {
  return new OpFailed(err.reason, err.message);
}

/** An op the socket lost: never sent, or not answered before it closed. */
export function opLost(message: string): OpFailed {
  return new OpFailed("disconnected", message);
}
