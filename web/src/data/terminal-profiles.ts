// A machine's terminal profiles on the page (ADR 0117): the list a node
// keeps, the rows' marks, and the calls that read, save and launch them.
//
// [Decision Log]
// - Purpose: Paseo's "Terminal profiles" rows in New tab and its host
//   Settings › Terminals list.
// - Reference: Paseo's app reads `terminalProfiles` from the daemon's config
//   and shows its four defaults when absent (`protocol/src/
//   terminal-profiles.ts` 19-30, 316-324). A save patches the whole list
//   (`host-page.tsx` 1438-1443). A row's mark is `icon ?? guess(base
//   name)` (108-132).
// - Choice: the node resolves and launches by id (ADR 0117, `terminal.create`'s
//   `profile`), so the page never substitutes or adopts. It needs the list's
//   names and marks only, plus the shipped four for a node that kept none.
//   The catalogue and the shipped list are copies of the Rust ones
//   (`vessel_proto::terminal_profiles`), kept by the tests below.
// - Cost: two copies of 48 names and 4 profiles. A provider added there is
//   added here.

import type { NodeKey } from "../wire/bridge";
import {
  type TerminalProfile,
  TERMINAL_PROFILES_PROTOCOL,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";

/** The number the fields came in; `null` while the integrator has not
 * assigned it. */
const SINCE: number | null = TERMINAL_PROFILES_PROTOCOL;

/** Whether a node speaking `protocol` keeps and launches profiles. */
export function profilesServed(protocol: number | null): boolean {
  return SINCE !== null && protocol !== null && protocol >= SINCE;
}

/** Paseo's defaults (`DEFAULT_TERMINAL_PROFILES`), as the node ships them. */
export const SHIPPED_PROFILES: readonly TerminalProfile[] = [
  {
    id: "claude",
    name: "Claude Code",
    command: "claude",
    args: ["{{{prompt}}}"],
    icon: "claude",
  },
  {
    id: "codex",
    name: "Codex",
    command: "codex",
    args: ["{{{prompt}}}"],
    icon: "codex",
  },
  {
    id: "opencode",
    name: "OpenCode",
    command: "opencode",
    args: ["--prompt={{{prompt}}}"],
    icon: "opencode",
  },
  { id: "pi", name: "Pi", command: "pi", args: ["{{{prompt}}}"], icon: "pi" },
];

/** Paseo's `KNOWN_PROVIDER_ICON_NAMES` (`provider-icon-names.ts` 1-60). */
const KNOWN_ICONS: ReadonlySet<string> = new Set([
  "antigravity",
  "claude",
  "codex",
  "copilot",
  "kiro",
  "minimax",
  "omp",
  "opencode",
  "pi",
  "agoragentic-acp",
  "amp-acp",
  "auggie",
  "autohand",
  "cline",
  "codebuddy-code",
  "codewhale",
  "cortex-code",
  "corust-agent",
  "crow-cli",
  "cursor",
  "deepagents",
  "devin",
  "dimcode",
  "dirac",
  "factory-droid",
  "fast-agent",
  "gemini",
  "gjc",
  "glm-acp-agent",
  "goose",
  "grok",
  "hermes",
  "hoplite",
  "junie",
  "kilo",
  "kimi",
  "minimax-code",
  "minion-code",
  "mistral-vibe",
  "nova",
  "poolside",
  "qoder",
  "qwen-code",
  "sigit",
  "stakpak",
  "traecli",
  "vtcode",
  "agy",
]);

/** Programs not named after their mark (`COMMAND_ICON_ALIASES`). */
const ALIASES: ReadonlyMap<string, string> = new Map([
  ["cursor-agent", "cursor"],
]);

/** The marks the page has artwork for. */
const DRAWN = ["claude", "codex", "opencode"] as const;
export type DrawnMark = (typeof DRAWN)[number];

/** What a profile's row draws: a vendor's mark, the terminal glyph (no
 * name, or a name the page has no artwork for), or the generic agent mark
 * (a set name outside the catalogue: Paseo's Bot). */
export type ProfileMark =
  | { kind: "mark"; name: DrawnMark }
  | { kind: "terminal" }
  | { kind: "generic" };

/** A command's base name: past the last `/` or `\`, lowercased, cut at
 * its first `.` unless it starts with one (`getCommandBaseName`). */
export function baseName(command: string): string {
  const last = command.split(/[/\\]/).at(-1) ?? command;
  const lower = last.toLowerCase();
  const dot = lower.indexOf(".");
  return dot > 0 ? lower.slice(0, dot) : lower;
}

/** The catalogue name `command` guesses (`guessTerminalProfileIcon`). */
export function guessIcon(command: string): string | null {
  const base = baseName(command);
  return KNOWN_ICONS.has(base) ? base : (ALIASES.get(base) ?? null);
}

function drawn(name: string): DrawnMark | null {
  return DRAWN.find((mark) => mark === name) ?? null;
}

/** The row's mark: its `icon`, else the command's guess. */
export function profileMark(profile: TerminalProfile): ProfileMark {
  const set = profile.icon;
  if (set !== undefined && !KNOWN_ICONS.has(set)) {
    return { kind: "generic" };
  }
  const name = set ?? guessIcon(profile.command);
  const art = name === null ? null : drawn(name);
  return art === null ? { kind: "terminal" } : { kind: "mark", name: art };
}

/** What a row shows under its name: the command and its arguments. */
export function commandLine(profile: TerminalProfile): string {
  return [profile.command, ...profile.args].join(" ");
}

/** `node`'s profiles: the kept list, or the shipped four when it kept
 * none. */
export function readProfiles(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<readonly TerminalProfile[]>> {
  return attempt(async () => {
    const settings = await fleet.bridge.call(node, "node.settings", {});
    return settings.terminal_profiles ?? SHIPPED_PROFILES;
  });
}

/** Keep `profiles` as `node`'s whole list; answers what it kept. */
export function saveProfiles(
  fleet: Fleet,
  node: NodeKey,
  profiles: readonly TerminalProfile[],
): Promise<Outcome<readonly TerminalProfile[]>> {
  return attempt(async () => {
    const settings = await fleet.bridge.call(node, "node.set_settings", {
      terminal_profiles: [...profiles],
    });
    return settings.terminal_profiles ?? SHIPPED_PROFILES;
  });
}

/** Paseo's `parseArgsString`: whitespace splits, no quoting. */
export function parseArgs(text: string): string[] {
  return text.split(/\s+/).filter((part) => part.length > 0);
}

/** Paseo's `generateProfileId`: `profile_<base36 time>_<base36 random>`. */
export function newProfileId(now: number, random: number): string {
  return `profile_${now.toString(36)}_${random.toString(36).slice(2)}`;
}
