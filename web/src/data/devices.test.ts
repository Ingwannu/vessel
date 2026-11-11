import { describe, expect, it } from "vitest";

import type { DeviceInfo } from "../wire/proto";
import { deviceLine, pairedDevices } from "./devices";
import { fixtureId } from "../testing/ids";

const phone: DeviceInfo = {
  id: fixtureId("DeviceId", "d-1"),
  label: "phone",
  created_at: "2026-11-11T05:00:00Z",
  last_seen: "2026-11-11T06:57:00Z",
  scopes: ["read", "operate"],
};

describe("a paired device's row", () => {
  it("says what it may do and when it was seen and paired", () => {
    const now = Date.parse("2026-11-11T07:00:00Z");
    expect(deviceLine(phone, now)).toEqual([
      "Can read, run sessions",
      "seen 3m ago",
      "paired 2h ago",
    ]);
    expect(deviceLine({ ...phone, last_seen: null }, now)[1]).toBe(
      "never seen",
    );
  });

  it("lists only the devices that still have access", () => {
    const revoked = {
      ...phone,
      id: fixtureId("DeviceId", "d-2"),
      revoked_at: "2026-11-11T06:00:00Z",
    };
    expect(pairedDevices([phone, revoked])).toEqual([phone]);
  });
});
