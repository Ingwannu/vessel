import { describe, expect, it } from "vitest";

import { fixtureId } from "../testing/ids";
import type {
  ClientInstallation,
  Contribution,
  PluginCatalog,
} from "../wire/proto";
import {
  contextAt,
  type MachineCatalog,
  type PluginPlace,
  pluginCommands,
  pluginSlashCommands,
  resolveSlashCommand,
} from "./plugin-catalog";

const laptop = fixtureId("NodeKey", "laptop");
const server = fixtureId("NodeKey", "build-server");
const project = fixtureId("ProjectId", "p1");
const agent = fixtureId("SessionId", "s1");

const command = (
  id: string,
  context: "global" | "workspace" | "agent",
): Contribution => ({
  id,
  title: `Run ${id}`,
  content: {
    kind: "command",
    keywords: ["notes"],
    context,
    action: { kind: "invoke", method: `notes.${id}`, input: { id } },
  },
});

const slash: Contribution = {
  id: "remember",
  title: "Remember",
  content: {
    kind: "slash_command",
    name: "remember",
    description: "Keep a note",
    argument_hint: "what to keep",
    context: "agent",
    method: "notes.remember",
  },
};

const notes: ClientInstallation = {
  address: {
    node: fixtureId("NodeId", "node-1"),
    plugin: "notes",
    generation: fixtureId("RuntimeGeneration", "run-1"),
  },
  required_client: 37,
  document: {
    methods: ["notes.global", "notes.agent", "notes.remember"],
    contributions: [
      command("global", "global"),
      command("agent", "agent"),
      slash,
    ],
    themes: [],
  },
};

const held = (mayInvoke: boolean): MachineCatalog => {
  const catalog: PluginCatalog = {
    generation: 3,
    catalog: { node: fixtureId("NodeId", "node-1"), installations: [notes] },
    may_invoke: mayInvoke,
  };
  return { catalog };
};

const inSession: PluginPlace = {
  kind: "session",
  node: laptop,
  agent,
  workspace: { project },
};

describe("plugin contexts (Paseo: a workspace command needs a workspace)", () => {
  it("gives global anywhere and agent only in that machine's session", () => {
    expect(contextAt("global", { kind: "global" }, laptop)).toEqual({
      kind: "global",
    });
    expect(contextAt("agent", { kind: "global" }, laptop)).toBeNull();
    expect(contextAt("agent", inSession, server)).toBeNull();
    expect(contextAt("agent", inSession, laptop)).toEqual({
      kind: "agent",
      project,
      agent,
    });
    expect(
      contextAt("workspace", { ...inSession, workspace: null }, laptop),
    ).toBeNull();
  });
});

describe("plugin Command Center items", () => {
  it("lists what can run here, and nothing where the caller may not invoke", () => {
    const anywhere = pluginCommands(new Map([[laptop, held(true)]]), {
      kind: "global",
    });
    expect(anywhere.map((item) => item.title)).toEqual(["Run global"]);
    const here = pluginCommands(new Map([[laptop, held(true)]]), inSession);
    expect(here.map((item) => item.call.method)).toEqual([
      "notes.global",
      "notes.agent",
    ]);
    expect(here[1]?.call.context).toEqual({ kind: "agent", project, agent });
    expect(pluginCommands(new Map([[laptop, held(false)]]), inSession)).toEqual(
      [],
    );
  });
});

describe("plugin slash commands", () => {
  it("offers the session machine's commands and resolves /name args", () => {
    const catalogs = new Map([[laptop, held(true)]]);
    const commands = pluginSlashCommands(catalogs, laptop, inSession);
    expect(commands.map((item) => item.name)).toEqual(["remember"]);
    expect(pluginSlashCommands(catalogs, server, inSession)).toEqual([]);
    const resolved = resolveSlashCommand(
      "/remember  buy milk ",
      false,
      commands,
    );
    expect(resolved?.args).toBe("buy milk");
    expect(resolved?.command.call(resolved.args).input).toEqual({
      arguments: "buy milk",
    });
    expect(resolveSlashCommand("/remember x", true, commands)).toBeNull();
    expect(resolveSlashCommand("/other x", false, commands)).toBeNull();
    expect(resolveSlashCommand("remember x", false, commands)).toBeNull();
  });
});
