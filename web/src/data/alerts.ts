// What a notification setting means (Settings › Notifications; GOAT's
// "Thread notifications" modes), and what the browser allows on this
// launch's address.

import type { Alerts } from "../wire/bridge";

/** Whether `alerts` shows the system's notification. */
export function showsSystem(alerts: Alerts): boolean {
  return alerts === "system" || alerts === "system_and_sound";
}

/** Whether `alerts` plays a sound. */
export function playsSound(alerts: Alerts): boolean {
  return alerts === "sound" || alerts === "system_and_sound";
}

/** Whether the page has anything to tell while hidden, and so keeps
 * polling then. */
export function wantsNotices(settings: {
  alerts: Alerts;
  title: boolean;
}): boolean {
  return settings.alerts !== "off" || settings.title;
}

/** The browser's notification permission on this launch's address. */
export type Permission = "granted" | "default" | "denied" | "unsupported";

/** What Settings says of the permission. */
export function permissionWords(permission: Permission): string {
  switch (permission) {
    case "granted":
      return "Allowed for this launch.";
    case "default":
      return "Not asked in this launch.";
    case "denied":
      return "Blocked by the browser for this address.";
    case "unsupported":
      return "This browser cannot show notifications here.";
  }
}

/** The modes' names, in GOAT's words. */
export function alertsLabel(alerts: Alerts): string {
  switch (alerts) {
    case "off":
      return "Off";
    case "system":
      return "Notifications only";
    case "sound":
      return "Sound only";
    case "system_and_sound":
      return "Notifications with sound";
  }
}
