import { describe, expect, it } from "vitest";

import { profileFacts, routeFacts } from "./model-rows";

describe("a machine's models in Settings", () => {
  it("say a route's provider, models and API", () => {
    expect(
      routeFacts({
        name: "qwen-coder",
        provider: "openrouter",
        api: "openai_chat_completions",
        model: "qwen/qwen3-coder",
        small_model: "qwen/qwen3-coder-flash",
      }),
    ).toEqual([
      "openrouter",
      "qwen/qwen3-coder",
      "small: qwen/qwen3-coder-flash",
      "openai-chat-completions",
    ]);
  });

  it("say a profile's agent, where its models come from and what it sets", () => {
    expect(
      profileFacts({
        name: "claude-qwen",
        harness: "claude-code",
        sets: ["A", "B"],
        model: "qwen-coder",
      }),
    ).toEqual(["Claude Code", "models from qwen-coder", "sets 2 variables"]);
    expect(profileFacts({ name: "work", harness: "codex", sets: [] })).toEqual([
      "Codex",
      "the agent's own models",
    ]);
  });
});
