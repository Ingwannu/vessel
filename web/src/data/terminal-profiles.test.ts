import { describe, expect, it } from "vitest";

import type { TerminalProfile } from "../wire/proto";
import {
  baseName,
  commandLine,
  guessIcon,
  newProfileId,
  parseArgs,
  profileMark,
  profilesServed,
  SHIPPED_PROFILES,
} from "./terminal-profiles";

const kept = (command: string, icon?: string): TerminalProfile => ({
  id: "profile_a",
  name: "Kept",
  command,
  args: [],
  ...(icon === undefined ? {} : { icon }),
});

describe("terminal profiles on the page", () => {
  it("guess a mark from the whole base name, as the node does", () => {
    // The Rust cases (`terminal_profiles/adopt_tests.rs`).
    for (const [command, expected] of [
      ["claude", "claude"],
      ["Claude", "claude"],
      ["/usr/local/bin/codex", "codex"],
      ["C:\\tools\\Codex.CMD", "codex"],
      ["cursor-agent", "cursor"],
      ["agy", "agy"],
    ] as const) {
      expect(guessIcon(command)).toBe(expected);
    }
    for (const command of ["zsh", "claude-code", "my-codex", ".codex"]) {
      expect(guessIcon(command)).toBeNull();
    }
    expect(baseName(".hidden.sh")).toBe(".hidden.sh");
    expect(baseName("tool.v2.exe")).toBe("tool");
  });

  it("draw the vendor's mark, the terminal glyph or the generic one", () => {
    expect(profileMark(kept("claude"))).toEqual({
      kind: "mark",
      name: "claude",
    });
    expect(profileMark(kept("zsh", "codex"))).toEqual({
      kind: "mark",
      name: "codex",
    });
    // Known, but no artwork here: the terminal glyph, as Pi.
    expect(profileMark(kept("pi"))).toEqual({ kind: "terminal" });
    expect(profileMark(kept("cursor-agent"))).toEqual({ kind: "terminal" });
    expect(profileMark(kept("zsh"))).toEqual({ kind: "terminal" });
    expect(profileMark(kept("zsh", "not-a-provider"))).toEqual({
      kind: "generic",
    });
  });

  it("ship Paseo's four, and show a row's command line", () => {
    expect(SHIPPED_PROFILES.map((profile) => profile.id)).toEqual([
      "claude",
      "codex",
      "opencode",
      "pi",
    ]);
    expect(commandLine(SHIPPED_PROFILES[2] ?? kept("x"))).toBe(
      "opencode --prompt={{{prompt}}}",
    );
  });

  it("split arguments on whitespace and make Paseo-shaped ids", () => {
    expect(parseArgs("  --yolo   --model  x ")).toEqual([
      "--yolo",
      "--model",
      "x",
    ]);
    expect(parseArgs("   ")).toEqual([]);
    expect(newProfileId(1_739_308_800_000, 0.5)).toBe("profile_m70zjv9c_i");
    expect(newProfileId(36, 0.5)).toMatch(/^profile_10_[0-9a-z]*$/);
  });

  it("are served from protocol 45", () => {
    expect(profilesServed(null)).toBe(false);
    expect(profilesServed(44)).toBe(false);
    expect(profilesServed(45)).toBe(true);
  });
});
