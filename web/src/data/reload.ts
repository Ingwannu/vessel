// "Reload agent" (TRACKER 803, ADR 0110): Paseo's tab menu row
// (workspace-tab-menu.ts, `reload-agent`) and its handler
// (workspace-screen.tsx `handleReloadAgent`), over the node's
// `session.reload`.
//
// [Decision Log]
// - Purpose: a person restarts a session's agent so it picks up skills,
//   MCP servers and its login; the node also adds what the agent's own
//   history has beyond the log.
// - Where: Paseo has the row on an agent tab's menu; the page has no tab
//   menu, so the row sits in the session's actions menu, after "Copy agent
//   id", as Paseo's comes after its other non-destructive rows.
// - Who sees it: a structured session, unless its agent said it cannot
//   load a conversation or its machine is connected and speaks less than
//   `RELOAD_PROTOCOL`. Paseo offers it on every agent tab. A machine that
//   is not connected keeps the row, as Paseo's does: a press says "Host is
//   not connected" (Paseo's `workspace.terminal.hostDisconnected`), since
//   hiding it would move the menu under the person's pointer whenever the
//   link drops. A session whose agent has not named
//   a conversation yet is offered too: the page's copy of the info may not
//   have it yet, and the node's refusal ("its agent has no conversation
//   yet") is the failure toast. An ended session is offered: the node
//   starts it again (Paseo loads a closed agent).
// - Words and toasts are Paseo's (en.ts `workspace.tabs.menu.reloadAgent`,
//   `reloadAgentTooltip`, `toasts.reloadingAgent`, `reloadedAgent`,
//   `failedToReloadAgent`): "Reloading agent..." stays up until the answer
//   replaces it; a failure shows the node's message, else Paseo's line.

// `RELOAD_PROTOCOL` is generated from vessel-proto's into the wire
// declarations, so it stays equal to the Rust constant.
import { RELOAD_PROTOCOL, type SessionInfo } from "../wire/proto";

/** Paseo's words for the row and its toasts. */
export const RELOAD_WORDS = {
  label: "Reload agent",
  tooltip: "Reload agent to update skills, MCPs or login status.",
  reloading: "Reloading agent...",
  reloaded: "Reloaded agent",
  failed: "Failed to reload agent",
  disconnected: "Host is not connected",
} as const;

/** Whether the page offers "Reload agent" for `session` on a connection
 * speaking `protocol` (`null` while not connected: offered, and a press
 * says so). */
export function canReload(
  session: SessionInfo,
  protocol: number | null,
): boolean {
  if (protocol !== null && protocol < RELOAD_PROTOCOL) {
    return false;
  }
  if (session.spec.mode !== "structured") {
    return false;
  }
  return session.abilities?.resume !== false;
}

/** The failure toast's words: the node's message, as Paseo shows an
 * `Error`'s, else Paseo's own line. */
export function reloadFailure(message: string): string {
  return message.trim() === "" ? RELOAD_WORDS.failed : message;
}
