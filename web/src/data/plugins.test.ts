import { describe, expect, it } from "vitest";

import { PLUGINS_PROTOCOL, type PluginListItem } from "../wire/proto";
import {
  changedWords,
  installSource,
  logHead,
  pluginPageState,
  pluginSourceLine,
  speaksPlugins,
  statusLook,
} from "./plugins";

const ready = {
  connected: true,
  supported: true,
  loading: false,
  error: false,
  pluginCount: 1,
};

const night: PluginListItem = {
  id: "night",
  name: "Night",
  source: "directory",
  enabled: true,
  status: "running",
};

describe("plugin page state (Paseo's resolvePluginPageState)", () => {
  it("checks offline, then support, loading, error and count, in order", () => {
    expect(pluginPageState({ ...ready, connected: false })).toBe("offline");
    expect(pluginPageState({ ...ready, supported: false, loading: true })).toBe(
      "unsupported",
    );
    expect(pluginPageState({ ...ready, loading: true, error: true })).toBe(
      "loading",
    );
    expect(pluginPageState({ ...ready, error: true })).toBe("error");
    expect(pluginPageState({ ...ready, pluginCount: 0 })).toBe("empty");
    expect(pluginPageState(ready)).toBe("ready");
  });

  it("needs the plugin protocol", () => {
    expect(speaksPlugins(PLUGINS_PROTOCOL)).toBe(true);
    expect(speaksPlugins(PLUGINS_PROTOCOL - 1)).toBe(false);
  });
});

describe("install field (Paseo's plugin-install-form-model)", () => {
  it("sends only a trimmed, non-empty source", () => {
    expect(installSource("   ")).toBeNull();
    expect(installSource("  ~/plugins/night ")).toBe("~/plugins/night");
  });
});

describe("row words", () => {
  it("gives Paseo's status words and badge tones", () => {
    expect(statusLook("running")).toEqual({ word: "running", tone: "ok" });
    expect(statusLook("failed")).toEqual({ word: "failed", tone: "danger" });
    expect(statusLook("disabled")).toEqual({
      word: "disabled",
      tone: "neutral",
    });
  });

  it("says each change as Paseo's feedback", () => {
    expect(changedWords("install", "night")).toBe("Installed night");
    expect(changedWords("reload", "night")).toBe("Reloaded night");
    expect(changedWords("remove", "night")).toBe("Removed night");
  });

  it("shows the source only when the node gave the path", () => {
    expect(pluginSourceLine(night)).toBeNull();
    expect(pluginSourceLine({ ...night, path: "/home/a/night" })).toBe(
      "Directory: /home/a/night",
    );
  });

  it("heads a log entry with its time and where it came from", () => {
    expect(logHead("2026-11-19T18:00:00Z", "host")).toBe(
      "2026-11-19T18:00:00Z vessel",
    );
    expect(logHead("2026-11-19T18:00:00Z", "stderr")).toBe(
      "2026-11-19T18:00:00Z stderr",
    );
  });
});
