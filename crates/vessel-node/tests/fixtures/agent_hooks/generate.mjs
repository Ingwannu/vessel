// Regenerates the Paseo fixtures of vessel-node's agent hook tests
// (src/agent_hooks/tests.rs) from Paseo's own installer, so the tests read
// exactly what Paseo writes into a person's agent configuration.
//
// [Decision Log]
// - Purpose: Paseo and Vessel run side by side on one machine and both
//   install hooks into the same ~/.claude/settings.json and ~/.codex/hooks.json.
//   Vessel must leave every Paseo entry byte-identical and in place, and
//   Paseo must not take Vessel's entries for its own.
// - Choice: the fixtures are Paseo 0.15.4's installer output over a person's
//   file, not a hand-written imitation and never a copy of a real file.
// - Safety: this writes only under the scratch folder it is given. It passes
//   that folder to Paseo as `configDir`, so Paseo never resolves a home, and
//   it refuses to run while any agent home variable is set.
//
// Run, with a new empty scratch folder outside every real agent home:
//   env -i PATH="$PATH" HOME=<scratch>/home node generate.mjs \
//     <@getpaseo/server 0.15.4 package folder> <scratch> <this folder>
//
// It writes, for claude and codex:
// - paseo-0.15.4-<agent>.json: a person's file after Paseo's install.
// - paseo-0.15.4-<agent>.paseo-after-vessel.json: that file with Vessel's
//   entries added, then Paseo's install run again over it.
// - paseo-0.15.4-<agent>.paseo-after-vessel.vessel-off.json: the same with
//   Vessel's entries taken out, as Paseo itself would format it.
// It fails unless Paseo's install and uninstall both keep Vessel's entries.

import { mkdirSync, readFileSync, readdirSync, writeFileSync, copyFileSync } from "node:fs";
import path from "node:path";

const [paseoPackage, scratch, outDir] = process.argv.slice(2);
if (!paseoPackage || !scratch || !outDir || !path.isAbsolute(scratch)) {
  throw new Error("usage: generate.mjs <paseo server package> <absolute scratch> <out dir>");
}
for (const name of ["CLAUDE_CONFIG_DIR", "CODEX_HOME", "OPENCODE_CONFIG_DIR", "XDG_CONFIG_HOME"]) {
  if (process.env[name]) throw new Error(`${name} is set; run under env -i`);
}
if (process.env.HOME !== path.join(scratch, "home")) {
  throw new Error("HOME must be <scratch>/home");
}
if (readdirSync(scratch).some((entry) => entry !== "home")) {
  throw new Error("the scratch folder must be new and empty");
}
const version = JSON.parse(readFileSync(path.join(paseoPackage, "package.json"), "utf8")).version;
if (version !== "0.15.4") throw new Error(`expected Paseo 0.15.4, found ${version}`);

const hooks = path.join(paseoPackage, "dist/server/terminal/agent-hooks");
const { installAgentHooks, uninstallAgentHooks } = await import(
  path.join(hooks, "agent-hook-installer.js")
);
const { claudeAgentHookProvider } = await import(path.join(hooks, "claude/claude.js"));
const { codexAgentHookProvider } = await import(path.join(hooks, "codex/codex.js"));

// A person's files before Paseo, formatted as the agents write them.
const personal = {
  claude: {
    $schema: "https://json.schemastore.org/claude-code-settings.json",
    model: "opus",
    permissions: { allow: ["Bash(git status)"] },
    hooks: {
      Stop: [{ matcher: "", hooks: [{ type: "command", command: "notify-send 'Claude finished'" }] }],
      PreToolUse: [{ matcher: "Bash", hooks: [{ type: "command", command: "~/bin/audit-bash" }] }],
    },
    statusLine: { type: "command", command: "~/.claude/statusline.sh" },
  },
  codex: {
    hooks: {
      PostToolUse: [{ matcher: "", hooks: [{ type: "command", command: "~/bin/log-tool", timeout: 5 }] }],
    },
  },
};

// Vessel's commands, as src/agent_hooks.rs writes them (tests.rs checks the
// same text through the fixtures).
const vesselShell = (agent, event) =>
  `if [ -n "$VESSEL_TERMINAL_ACTIVITY_ID" ]; then "\${VESSEL_HOOK_CLI:-vessel}" hooks "${agent}" "${event}" 2>/dev/null || true; fi`;
const vesselWindows = (agent, event) =>
  `if defined VESSEL_TERMINAL_ACTIVITY_ID (if defined VESSEL_HOOK_CLI ("%VESSEL_HOOK_CLI%" hooks "${agent}" "${event}" 2>NUL & exit /b 0) else (vessel hooks "${agent}" "${event}" 2>NUL & exit /b 0)) else (exit /b 0)`;
const vesselEntry = (agent, event) => ({
  matcher: "",
  hooks: [
    {
      type: "command",
      command: vesselShell(agent, event),
      ...(agent === "codex" ? { commandWindows: vesselWindows(agent, event) } : {}),
      timeout: 10,
    },
  ],
});
const isVessel = (agent, entry) =>
  (entry.hooks ?? []).some((hook) =>
    [vesselShell, vesselWindows].some((form) =>
      events[agent].some((event) => [hook.command, hook.commandWindows].includes(form(agent, event))),
    ),
  );
const events = {
  claude: ["UserPromptSubmit", "Stop", "StopFailure", "SessionEnd", "Notification"],
  codex: ["UserPromptSubmit", "PreToolUse", "PostToolUse", "PermissionRequest", "Stop"],
};
const stringify = (config) => `${JSON.stringify(config, null, 2)}\n`;

const providers = { claude: claudeAgentHookProvider, codex: codexAgentHookProvider };
const fileOf = { claude: "settings.json", codex: "hooks.json" };
mkdirSync(outDir, { recursive: true });
for (const agent of ["claude", "codex"]) {
  const configDir = path.join(scratch, agent);
  mkdirSync(configDir, { recursive: true });
  const file = path.join(configDir, fileOf[agent]);
  const options = { configDir, env: {}, homeDir: path.join(scratch, "home") };
  writeFileSync(file, stringify(personal[agent]));

  installAgentHooks(providers[agent], options);
  copyFileSync(file, path.join(outDir, `paseo-0.15.4-${agent}.json`));

  // Vessel's install, as its tests expect it: its entry last in each list.
  const withVessel = JSON.parse(readFileSync(file, "utf8"));
  for (const event of events[agent]) {
    withVessel.hooks[event] = [...(withVessel.hooks[event] ?? []), vesselEntry(agent, event)];
  }
  writeFileSync(file, stringify(withVessel));

  installAgentHooks(providers[agent], options);
  const after = JSON.parse(readFileSync(file, "utf8"));
  for (const event of events[agent]) {
    if (!after.hooks[event].some((entry) => isVessel(agent, entry))) {
      throw new Error(`Paseo's install dropped Vessel's ${agent} ${event}`);
    }
  }
  copyFileSync(file, path.join(outDir, `paseo-0.15.4-${agent}.paseo-after-vessel.json`));
  const off = structuredClone(after);
  for (const event of events[agent]) {
    off.hooks[event] = off.hooks[event].filter((entry) => !isVessel(agent, entry));
    if (off.hooks[event].length === 0) delete off.hooks[event];
  }
  writeFileSync(
    path.join(outDir, `paseo-0.15.4-${agent}.paseo-after-vessel.vessel-off.json`),
    stringify(off),
  );

  uninstallAgentHooks(providers[agent], options);
  const uninstalled = JSON.parse(readFileSync(file, "utf8"));
  for (const event of events[agent]) {
    if (!uninstalled.hooks[event].some((entry) => isVessel(agent, entry))) {
      throw new Error(`Paseo's uninstall dropped Vessel's ${agent} ${event}`);
    }
  }
  console.log(`${agent}: Paseo's install and uninstall keep Vessel's entries`);
}
