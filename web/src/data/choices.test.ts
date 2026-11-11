import { describe, expect, it } from "vitest";

import type { BookNode, NodeStatus } from "../wire/bridge";
import type { ModelRoute, ProfileInfo } from "../wire/proto";
import {
  harnessOptions,
  moveOptions,
  nodeOptions,
  profileOptions,
  routeOptions,
} from "./choices";

const plain: ProfileInfo = { name: "claude", harness: "claude-code", sets: [] };
const routed: ProfileInfo = {
  name: "claude-qwen",
  label: "Claude Code on Qwen",
  harness: "claude-code",
  sets: [],
  model: "qwen-coder",
};
const qwen: ModelRoute = {
  name: "qwen-coder",
  provider: "qwen",
  api: "openai_chat_completions",
  model: "qwen/qwen3-coder",
};
const kimi: ModelRoute = {
  name: "kimi",
  provider: "moonshot",
  api: "openai_chat_completions",
  model: "kimi-k2",
};

describe("choices", () => {
  it("offers only connected nodes, with their state", () => {
    const book: BookNode[] = [
      {
        key: "a",
        label: "laptop",
        address: "x",
        transport: "ws",
        default: true,
        id: null,
      },
      {
        key: "b",
        label: "gpu-box",
        address: "y",
        transport: "ws",
        default: false,
        id: null,
      },
    ];
    const statuses: Record<string, NodeStatus> = {
      b: { state: "failed", message: "refused" },
    };
    expect(nodeOptions(book, statuses)).toEqual([
      { value: "a", label: "laptop", detail: "connecting", disabled: true },
      { value: "b", label: "gpu-box", detail: "offline", disabled: true },
    ]);
  });

  it("names harnesses as the page does", () => {
    expect(
      harnessOptions([
        {
          kind: "claude-code",
          path: "/bin/claude",
          version: null,
          structured: { state: "available" },
        },
      ]),
    ).toEqual([{ value: "claude-code", label: "claude" }]);
  });

  it("lists the default profile first, a route as a profile's detail", () => {
    expect(profileOptions([plain, routed])).toEqual([
      { value: "", label: "default" },
      { value: "claude", label: "claude" },
      {
        value: "claude-qwen",
        label: "Claude Code on Qwen",
        detail: "qwen-coder",
      },
    ]);
  });

  it("moves a routed session only to a profile on its route", () => {
    const options = moveOptions([plain, routed], "qwen-coder");
    expect(options.map((option) => [option.value, option.disabled])).toEqual([
      ["", true],
      ["claude", true],
      ["claude-qwen", false],
    ]);
    const unrouted = moveOptions([plain, routed], null);
    expect(unrouted.map((option) => option.disabled)).toEqual([
      false,
      false,
      true,
    ]);
  });

  it("groups routes by provider and disables one no profile names", () => {
    expect(routeOptions([qwen, kimi], [plain, routed])).toEqual([
      {
        value: "",
        label: "harness default",
        group: "The harness's own provider",
      },
      {
        value: "qwen-coder",
        label: "qwen-coder",
        detail: "qwen/qwen3-coder",
        disabled: false,
        group: "qwen",
      },
      {
        value: "kimi",
        label: "kimi",
        detail: "kimi-k2 · no profile",
        disabled: true,
        group: "moonshot",
      },
    ]);
  });
});
