import { describe, expect, test } from "vitest";

import type { AgentCommand } from "../wire/proto";
import {
  applyCommand,
  applyFile,
  matchCommands,
  pathParts,
  triggerAt,
  withMention,
} from "./suggest";

describe("what the caret is in", () => {
  test("`/` only at the prompt's start", () => {
    expect(triggerAt("/rev", 4)).toEqual({
      kind: "command",
      query: "rev",
      start: 0,
      end: 4,
    });
    expect(triggerAt("see /rev", 8)).toBeNull();
    expect(triggerAt("/", 1)).toMatchObject({ kind: "command", query: "" });
  });

  test("`@` at a word's start, anywhere", () => {
    expect(triggerAt("look at @src/ma please", 15)).toEqual({
      kind: "file",
      query: "src/ma",
      start: 8,
      end: 15,
    });
    // The word runs past the caret: choosing replaces all of it.
    expect(triggerAt("@srcmain", 4)).toMatchObject({
      query: "src",
      end: 8,
    });
    expect(triggerAt("mail@example", 12)).toBeNull();
    expect(triggerAt("plain words", 5)).toBeNull();
  });
});

const COMMANDS: AgentCommand[] = [
  { name: "review", description: "Review the changes" },
  { name: "compact", description: "Summarise the conversation", hint: "focus" },
  { name: "pre-review", description: "Before a review" },
  { name: "init", description: "Write a guide to review later" },
];

test("names starting with the word first, then holding it, then descriptions", () => {
  expect(matchCommands(COMMANDS, "rev").map((command) => command.name)).toEqual(
    ["review", "pre-review", "init"],
  );
  expect(matchCommands(COMMANDS, "")).toHaveLength(4);
  expect(matchCommands(COMMANDS, "zzz")).toEqual([]);
});

describe("choosing", () => {
  test("a command replaces the word and leaves a space", () => {
    const trigger = triggerAt("/rev", 4);
    expect(trigger).not.toBeNull();
    if (trigger !== null) {
      expect(applyCommand("/rev", trigger, "review")).toEqual({
        text: "/review ",
        caret: 8,
      });
    }
  });

  test("a file, quoted when it holds a space, before what follows", () => {
    const text = "open @sr and fix";
    const trigger = triggerAt(text, 8);
    expect(trigger).not.toBeNull();
    if (trigger !== null) {
      expect(applyFile(text, trigger, "src/main.rs")).toEqual({
        text: "open @src/main.rs and fix",
        caret: 18,
      });
      expect(applyFile(text, trigger, "docs/a b.md").text).toBe(
        'open @"docs/a b.md" and fix',
      );
    }
  });
});

describe("the Explorer's Add to chat", () => {
  test("names the file as `@` does, a word of its own", () => {
    expect(withMention("", "src/main.rs")).toBe("@src/main.rs ");
    expect(withMention("  ", "src/main.rs")).toBe("@src/main.rs ");
    expect(withMention("look at", "src/main.rs")).toBe("look at @src/main.rs ");
    expect(withMention("look at\n", "docs/a b.md")).toBe(
      'look at\n@"docs/a b.md" ',
    );
  });
});

test("a found path: its folder dimmed, the name's matches marked", () => {
  const parts = pathParts({ path: "src/data/mod.rs", positions: [0, 9, 10] });
  expect(parts.folder).toBe("src/data/");
  expect(parts.name).toBe("mod.rs");
  expect([...parts.marks]).toEqual([0, 1]);
});
