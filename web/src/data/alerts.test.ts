import { expect, test } from "vitest";

import { alertsLabel, playsSound, showsSystem, wantsNotices } from "./alerts";

test("each mode's parts", () => {
  expect([
    showsSystem("system_and_sound"),
    playsSound("system_and_sound"),
  ]).toEqual([true, true]);
  expect([showsSystem("sound"), playsSound("sound")]).toEqual([false, true]);
  expect([showsSystem("off"), playsSound("off")]).toEqual([false, false]);
  expect(alertsLabel("system")).toBe("Notifications only");
});

test("a hidden page keeps polling only when something would be told", () => {
  expect(wantsNotices({ alerts: "off", title: false })).toBe(false);
  expect(wantsNotices({ alerts: "off", title: true })).toBe(true);
  expect(wantsNotices({ alerts: "sound", title: false })).toBe(true);
});
