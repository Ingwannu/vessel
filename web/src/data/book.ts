// The book's edits from the page (track E's ops), as typed calls: each
// gives the screen an outcome, and leaves the fleet's state as the book
// now is, so the lists redraw without a second round trip.
//
// The pairing link is a credential until it is redeemed: it goes from the
// field to the op and nowhere else. It is never put in an address, a log,
// a toast or a test's failure message (the bridge's `SecretText` keeps it
// out of the server's logs; this file keeps it out of the page's).

import type { BookNode, NodeKey } from "../wire/bridge";
import type { PairingOffer } from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";

/** What `inspect` read from a link, without sending anything. */
export interface Offer {
  offer: PairingOffer;
  /** The fingerprint of the machine's key, to compare with what that
   * machine shows (`vessel node status`). */
  fingerprint: string;
}

/** Read `link` (nothing is sent to the machine it names). */
export async function inspectLink(
  fleet: Fleet,
  link: string,
): Promise<Outcome<Offer>> {
  return attempt(async () => {
    const result = await fleet.bridge.op({ op: "inspect", link: link.trim() });
    if (result.op !== "inspect") {
      throw new Error("vessel ui answered another op");
    }
    return { offer: result.offer, fingerprint: result.fingerprint };
  });
}

/** Pair with the machine `link` names and add it to the book as `name`;
 * then read the book again, which connects it. */
export async function pairMachine(
  fleet: Fleet,
  name: string,
  link: string,
): Promise<Outcome<BookNode>> {
  const outcome = await attempt(async () => {
    const result = await fleet.bridge.op({
      op: "pair",
      name,
      link: link.trim(),
    });
    if (result.op !== "pair") {
      throw new Error("vessel ui answered another op");
    }
    return result.node;
  });
  if (outcome.ok) {
    void fleet.loadBook();
  }
  return outcome;
}

/** The edits that answer with the whole book. */
type BookEdit =
  | { op: "forget"; node: NodeKey }
  | { op: "rename"; node: NodeKey; name: string }
  | { op: "default"; node: NodeKey };

async function edit(
  fleet: Fleet,
  change: BookEdit,
): Promise<Outcome<readonly BookNode[]>> {
  const outcome = await attempt(async () => {
    const result = await fleet.bridge.op(change);
    switch (result.op) {
      case "forget":
      case "rename":
      case "default":
        return result.nodes;
      case "nodes":
      case "retry":
      case "inspect":
      case "pair":
      case "qr":
      case "preferences":
      case "prefer":
      case "drafts":
      case "draft":
      case "save_draft":
      case "check_submit":
        throw new Error("vessel ui answered another op");
    }
  });
  if (outcome.ok) {
    await fleet.takeBook(outcome.value);
  }
  return outcome;
}

/** Remove `node` from the book; its connection closes. Its credential
 * file stays, as `vessel nodes remove` leaves it. */
export function forgetMachine(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<readonly BookNode[]>> {
  return edit(fleet, { op: "forget", node });
}

/** Call `node` `name` in the book. */
export function renameMachine(
  fleet: Fleet,
  node: NodeKey,
  name: string,
): Promise<Outcome<readonly BookNode[]>> {
  return edit(fleet, { op: "rename", node, name });
}

/** Make `node` the one commands use when none is named. */
export function makeDefault(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<readonly BookNode[]>> {
  return edit(fleet, { op: "default", node });
}

/** Try connecting to `node` now, rather than at its next retry. */
export async function retryMachine(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<null>> {
  return attempt(async () => {
    await fleet.bridge.op({ op: "retry", node });
    return null;
  });
}
