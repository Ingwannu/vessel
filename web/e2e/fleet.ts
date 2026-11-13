// A small fleet for the end-to-end tests and the screenshots: real
// `vesseld` nodes in temporary directories with the testkit's fake
// harnesses (`fake-tui` as `claude`/`codex`, `fake-acp` as their ACP
// adapters), a node book naming them, and `vessel ui` serving the built
// page. Nothing touches the machine's own account: every process gets a
// temporary home, data and config directory, and the nodes listen on
// loopback ports chosen by the system.
//
// laptop also routes models (milestone 5): `fake-provider` stands in for
// an OpenAI-compatible provider on a loopback port, `models.toml` names it
// under two routes, and profile `claude-qwen` runs Claude Code on one of
// them, so a `model <text>` prompt to the fakes is a real call through
// the node's proxy. The provider and model names are only labels; nothing
// leaves the machine, and the key is made up for the run.
//
// The Rust binaries are built beforehand (`cargo build -p vessel-cli -p
// vessel-node -p vessel-testkit --bins`, with the bundle embedded); their
// directory comes from `VESSEL_TARGET_DIR` or `cargo metadata`, and the
// build (debug, or release with `VESSEL_CARGO_PROFILE=release`) from
// `cargoProfile`.

import { type ChildProcess, execFileSync, spawn } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { randomBytes } from "node:crypto";
import { connect, createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { addProjects, makeRepos } from "./repos";

export interface FleetEnv {
  /** The UI's address with the launch's token. */
  url: string;
  root: string;
  /** The `vessel` binary, for setting things up the way a user would. */
  vessel: string;
  pids: number[];
  nodes: Record<string, { port: number; data: string }>;
}

const EXE = process.platform === "win32" ? ".exe" : "";

function targetDir(): string {
  const given = process.env["VESSEL_TARGET_DIR"];
  if (given !== undefined && given !== "") {
    return given;
  }
  const metadata = execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--no-deps"],
    {
      cwd: join(import.meta.dirname, ".."),
      encoding: "utf8",
    },
  );
  const parsed: unknown = JSON.parse(metadata);
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("target_directory" in parsed) ||
    typeof parsed.target_directory !== "string"
  ) {
    throw new Error("cargo metadata names no target directory");
  }
  return parsed.target_directory;
}

/** Which build of the Rust binaries the fleet runs: `debug` (cargo's
 * dev profile, what the tests build) unless `VESSEL_CARGO_PROFILE` says
 * `release` (`cargo build --release ...`), which the page's performance
 * measurement (perf.ts) may want so that a debug node is not what it
 * measures. */
export function cargoProfile(): "debug" | "release" {
  const given = process.env["VESSEL_CARGO_PROFILE"];
  if (given === undefined || given === "" || given === "debug") {
    return "debug";
  }
  if (given === "release") {
    return "release";
  }
  throw new Error("VESSEL_CARGO_PROFILE is debug or release");
}

function binary(name: string): string {
  const path = join(targetDir(), cargoProfile(), `${name}${EXE}`);
  if (!existsSync(path)) {
    throw new Error(`${path} is missing: build the Rust binaries first`);
  }
  return path;
}

/** Whether something accepts a connection on loopback's `port`. */
async function accepts(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const socket = connect(port, "127.0.0.1");
    socket.once("connect", () => {
      socket.destroy();
      resolve(true);
    });
    socket.once("error", () => {
      socket.destroy();
      resolve(false);
    });
  });
}

async function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port =
        typeof address === "object" && address !== null ? address.port : 0;
      server.close(() => {
        resolve(port);
      });
    });
    server.on("error", reject);
  });
}

async function until(
  what: string,
  ready: () => boolean | Promise<boolean>,
  ms = 30_000,
): Promise<void> {
  const deadline = Date.now() + ms;
  while (!(await ready())) {
    if (Date.now() > deadline) {
      throw new Error(`timed out waiting for ${what}`);
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

/** The fake harnesses under the names a node looks harnesses up by. */
function installFakes(dir: string): void {
  mkdirSync(dir, { recursive: true });
  const tui = binary("fake-tui");
  const acp = binary("fake-acp");
  const links: [string, string][] = [
    ["claude", tui],
    ["claude-agent-acp", acp],
    ["codex", tui],
    ["codex-acp", acp],
  ];
  for (const [name, target] of links) {
    if (process.platform === "win32") {
      writeFileSync(
        join(dir, `${name}.cmd`),
        `@echo off\r\n"${target}" %*\r\nexit /b %ERRORLEVEL%\r\n`,
      );
    } else {
      symlinkSync(target, join(dir, name));
    }
  }
}

function spawnLogged(
  root: string,
  name: string,
  command: string,
  args: string[],
  env: NodeJS.ProcessEnv,
): ChildProcess {
  const child = spawn(command, args, {
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const log = join(root, `${name}.log`);
  child.stdout.on("data", (chunk: Buffer) => {
    writeFileSync(log, chunk, { flag: "a", mode: 0o600 });
  });
  child.stderr.on("data", (chunk: Buffer) => {
    writeFileSync(log, chunk, { flag: "a", mode: 0o600 });
  });
  return child;
}

/** laptop's model routes: `fake-provider` on loopback, the files naming
 * it, and its key as the node's secret, all before the node starts. */
async function routeModels(
  root: string,
  data: string,
  vessel: string,
  env: NodeJS.ProcessEnv,
): Promise<ChildProcess> {
  const key = `fake-key-${randomBytes(12).toString("hex")}`;
  const provider = spawnLogged(root, "provider", binary("fake-provider"), [], {
    ...env,
    FAKE_PROVIDER_KEY: key,
  });
  const log = join(root, "provider.log");
  let port = "";
  await until("fake-provider's port", () => {
    const match = existsSync(log)
      ? /^listening (\d+)/m.exec(readFileSync(log, "utf8"))
      : null;
    port = match?.[1] ?? "";
    return port !== "";
  });
  // The node refuses a models.toml others can read.
  mkdirSync(data, { recursive: true, mode: 0o700 });
  writeFileSync(
    join(data, "models.toml"),
    [
      "[providers.openrouter]",
      'api = "openai-chat-completions"',
      `base_url = "http://127.0.0.1:${port}/v1"`,
      'key = { secret = "OPENROUTER_API_KEY" }',
      "",
      "[providers.ollama]",
      'api = "openai-chat-completions"',
      `base_url = "http://127.0.0.1:${port}/v1"`,
      "",
      "[models.qwen-coder]",
      'provider = "openrouter"',
      'model = "qwen/qwen3-coder"',
      'small_model = "qwen/qwen3-coder-flash"',
      "",
      "[models.gpt-oss]",
      'provider = "ollama"',
      'model = "gpt-oss:20b"',
      "",
    ].join("\n"),
    { mode: 0o600 },
  );
  writeFileSync(
    join(data, "profiles.toml"),
    [
      "[profiles.claude-qwen]",
      'harness = "claude"',
      'model = "qwen-coder"',
      'label = "Claude Code on Qwen"',
      "",
    ].join("\n"),
    { mode: 0o600 },
  );
  execFileSync(
    vessel,
    ["--data-dir", data, "fleet", "secret", "set", "OPENROUTER_API_KEY"],
    { env, input: key },
  );
  return provider;
}

/** Start the nodes and the UI. */
export async function startFleet(): Promise<FleetEnv> {
  const root = mkdtempSync(join(tmpdir(), "vessel-e2e-"));
  const fakes = join(root, "bin");
  installFakes(fakes);
  const config = join(root, "config");
  const uiData = join(root, "ui-data");
  mkdirSync(config, { recursive: true });
  mkdirSync(uiData, { recursive: true });
  // The fakes and the system's own directories only: a harness installed
  // for the machine's user (in ~/.local/bin, say) must not be found.
  const system =
    process.platform === "win32"
      ? `${process.env["SYSTEMROOT"] ?? "C:\\Windows"}\\System32`
      : "/usr/local/bin:/usr/bin:/bin";
  const base: NodeJS.ProcessEnv = {
    PATH: `${fakes}${process.platform === "win32" ? ";" : ":"}${system}`,
    SYSTEMROOT: process.env["SYSTEMROOT"],
    RUST_LOG: "info",
  };
  const pids: number[] = [];
  const nodes: FleetEnv["nodes"] = {};
  const vessel = binary("vessel");
  for (const name of ["laptop", "gpu-box", "build-server"]) {
    const port = await freePort();
    const data = join(root, name);
    const home = join(root, `${name}-home`);
    mkdirSync(home, { recursive: true });
    if (name === "laptop") {
      const provider = await routeModels(root, data, vessel, {
        ...base,
        HOME: home,
        USERPROFILE: home,
        VESSEL_CONFIG_DIR: join(home, "config"),
      });
      if (provider.pid !== undefined) {
        pids.push(provider.pid);
      }
    }
    const args = [
      "--data-dir",
      data,
      "--bind",
      `127.0.0.1:${port}`,
      "--name",
      name,
    ];
    if (name === "laptop") {
      // Pairing links name a direct route, so a device can pair here.
      args.push("--public", `127.0.0.1:${port}`);
    }
    const child = spawnLogged(root, name, binary("vesseld"), args, {
      ...base,
      HOME: home,
      USERPROFILE: home,
    });
    if (child.pid !== undefined) {
      pids.push(child.pid);
    }
    nodes[name] = { port, data };
    await until(`${name}'s token`, () => existsSync(join(data, "token")));
    // The token is written before the listener is bound, and the CLI
    // talks to laptop before the UI starts (the projects, below).
    await until(`${name}'s listener`, () => accepts(port));
  }
  const cli = {
    ...base,
    HOME: root,
    USERPROFILE: root,
    VESSEL_CONFIG_DIR: config,
    VESSEL_DATA_DIR: uiData,
  };
  for (const [name, node] of Object.entries(nodes)) {
    const args = [
      "nodes",
      "add",
      name,
      `ws://127.0.0.1:${node.port}`,
      "--token-file",
      join(node.data, "token"),
    ];
    if (name === "laptop") {
      args.push("--default");
    }
    execFileSync(vessel, args, { env: cli });
  }
  // Agents added on laptop by their command (protocol 11, ADR 0032): the
  // fake ACP agent as one that runs, as one needing a secret the node does
  // not have (its start is refused), a program that is not there, and the
  // fake again, slow to start.
  const fakeAcp = binary("fake-acp");
  const added: string[][] = [
    ["helper", "--label", "Helper", "--", fakeAcp],
    [
      "keyed",
      "--label",
      "Keyed agent",
      "--secret",
      "KEYED_TOKEN=KEYED_TOKEN",
      "--",
      fakeAcp,
    ],
    ["ghost", "--", join(root, "no-such-agent")],
    // One that takes four seconds to start (e2e/new-chat.spec.ts).
    ["slow", "--label", "Slow starter", "--", fakeAcp, "--slow-start", "4000"],
  ];
  for (const args of added) {
    execFileSync(vessel, ["agents", "add", ...args], { env: cli });
  }
  // Repositories for the code panel (milestone 11, track B-page), added
  // as laptop's projects (e2e/repos.ts).
  const repos = makeRepos(root, base);
  addProjects(vessel, cli, [repos.shop, repos.notes]);
  const ui = spawnLogged(
    root,
    "ui",
    vessel,
    ["ui", "--no-open", "--port", "0"],
    cli,
  );
  if (ui.pid !== undefined) {
    pids.push(ui.pid);
  }
  const log = join(root, "ui.log");
  let url = "";
  await until("vessel ui's address", () => {
    const match = existsSync(log)
      ? /Vessel UI: (http:\/\/\S+)/.exec(readFileSync(log, "utf8"))
      : null;
    url = match?.[1] ?? "";
    return url !== "";
  });
  return { url, root, vessel, pids, nodes };
}

/** Stop a process of the fleet (to show a node going away). */
export function stopProcess(pid: number): void {
  try {
    process.kill(pid);
  } catch {
    // Already gone.
  }
}

export function stopFleet(env: FleetEnv): void {
  for (const pid of env.pids) {
    stopProcess(pid);
  }
  // Give the processes a moment to let go of their files.
  setTimeout(() => {
    rmSync(env.root, { recursive: true, force: true });
  }, 500);
}

const ENV_FILE = join(import.meta.dirname, "..", "artifacts", "fleet.json");

export function saveEnv(env: FleetEnv): void {
  mkdirSync(join(import.meta.dirname, "..", "artifacts"), { recursive: true });
  writeFileSync(ENV_FILE, JSON.stringify(env), { mode: 0o600 });
}

/** Whether `value`, read back from ENV_FILE, is the fleet saveEnv wrote. */
function isFleetEnv(value: unknown): value is FleetEnv {
  return (
    typeof value === "object" &&
    value !== null &&
    "url" in value &&
    typeof value.url === "string" &&
    "root" in value &&
    typeof value.root === "string" &&
    "vessel" in value &&
    typeof value.vessel === "string" &&
    "pids" in value &&
    Array.isArray(value.pids) &&
    "nodes" in value &&
    typeof value.nodes === "object" &&
    value.nodes !== null
  );
}

export function loadEnv(): FleetEnv {
  const env: unknown = JSON.parse(readFileSync(ENV_FILE, "utf8"));
  if (!isFleetEnv(env)) {
    throw new Error(`${ENV_FILE} is not a fleet; run the global setup`);
  }
  return env;
}
