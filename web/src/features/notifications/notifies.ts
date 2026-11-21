// Whether this window tells notices as the system's notifications: Settings'
// alerts include them and the browser lets the page show them. Only such a
// window claims a notice and can be elected a claim's recipient (ADR 0089,
// "Turned on": Paseo elects only among clients subscribed with
// `notifications: true`, `websocket-server.ts:2884-2886`). A window that
// only sounds, or tells nothing, says so in its presence and never claims;
// it keeps ADR 0114's local path.

import { showsSystem } from "../../data/alerts";
import { prefs } from "../../data/prefs";

export function windowNotifies(): boolean {
  return (
    showsSystem(prefs.get().values.notifications.alerts) &&
    "Notification" in window &&
    Notification.permission === "granted"
  );
}
