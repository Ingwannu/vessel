// The devices paired with a machine (Settings › Devices): the calls, and
// what a device's row says. Moved out of the old Devices page, which made
// these calls and decisions inside its component.

import type { NodeKey } from "../wire/bridge";
import type { DeviceInfo, Invitation, Scope } from "../wire/proto";
import { ago } from "./format";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";

/** Ten minutes, as the CLI's `vessel node pair`. */
export const LINK_TTL_SECS = 600;

/** A link to show: the invitation, its QR code and its key's fingerprint. */
export interface Pairing {
  invitation: Invitation;
  svg: string;
  fingerprint: string;
}

/** The devices `node` knows, revoked ones included. */
export function listDevices(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<readonly DeviceInfo[]>> {
  return attempt(() => fleet.bridge.call(node, "device.list", null));
}

/** A new single-use link for a device called `label` (or what it calls
 * itself), with its QR code and fingerprint. */
export function inviteDevice(
  fleet: Fleet,
  node: NodeKey,
  label: string,
): Promise<Outcome<Pairing>> {
  return attempt(async () => {
    const invitation = await fleet.bridge.call(node, "device.invite", {
      label: label.trim() === "" ? null : label.trim(),
      ttl_secs: LINK_TTL_SECS,
    });
    const [qr, offer] = await Promise.all([
      fleet.bridge.op({ op: "qr", text: invitation.link }),
      fleet.bridge.op({ op: "inspect", link: invitation.link }),
    ]);
    return {
      invitation,
      svg: qr.op === "qr" ? qr.svg : "",
      fingerprint: offer.op === "inspect" ? offer.fingerprint : "",
    };
  });
}

/** Take `device`'s access to `node` away; its connections close. */
export function revokeDevice(
  fleet: Fleet,
  node: NodeKey,
  device: DeviceInfo,
): Promise<Outcome<null>> {
  return attempt(async () => {
    await fleet.bridge.call(node, "device.revoke", { device: device.id });
    return null;
  });
}

/** The devices that still have access. */
export function pairedDevices(
  devices: readonly DeviceInfo[],
): readonly DeviceInfo[] {
  return devices.filter(
    (device) => device.revoked_at === undefined || device.revoked_at === null,
  );
}

/** What a scope lets a device do, in a few words. */
export function scopeWord(scope: Scope): string {
  switch (scope) {
    case "read":
      return "read";
    case "operate":
      return "run sessions";
    case "admin":
      return "manage";
    case "sync":
      return "sync";
    case "relay":
      return "relay";
  }
}

/** A device's row's line: what it may do, when it was last seen, when it
 * was paired. */
export function deviceLine(
  device: DeviceInfo,
  now: number = Date.now(),
): string[] {
  const may = device.scopes.map(scopeWord).join(", ");
  const seen =
    device.last_seen === undefined || device.last_seen === null
      ? "never seen"
      : `seen ${ago(device.last_seen, now)} ago`;
  return [`Can ${may}`, seen, `paired ${ago(device.created_at, now)} ago`];
}
