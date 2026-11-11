// Sessions for the measurements (perf.ts), started and stopped through
// the `vessel` CLI against the fleet's nodes, as a person would, before
// the page opens: what is measured is the page drawing them, not their
// creation. Also what the fleet's processes cost, read from /proc.

import { execFile } from "node:child_process";
import { mkdirSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { promisify } from "node:util";

import type { FleetEnv } from "../fleet";
import { cliEnv } from "../fleet-home";

const run = promisify(execFile);

/** `vessel --node <node> <args>` as the fleet's user: what it printed,
 * both streams (a session's log prints its status lines on stderr).
 * Async, unlike fleet-home's `cli`, so hundreds can run side by side. */
export async function vesselOutput(
  env: FleetEnv,
  node: string,
  args: readonly string[],
): Promise<{ stdout: string; stderr: string }> {
  return run(env.vessel, ["--node", node, ...args], {
    env: cliEnv(env),
    encoding: "utf8",
    // A log of 20,000 entries is a few MB of text.
    maxBuffer: 256 * 1024 * 1024,
  });
}

/** `vessel --node <node> <args>`'s stdout. */
export async function vessel(
  env: FleetEnv,
  node: string,
  args: readonly string[],
): Promise<string> {
  return (await vesselOutput(env, node, args)).stdout;
}

/** A working directory under the fleet's root, made if missing. */
export function folder(env: FleetEnv, name: string): string {
  const path = join(env.root, "perf", name);
  mkdirSync(path, { recursive: true });
  return path;
}

/** Start a session of Claude Code (`fake-tui`, or `fake-acp` with
 * `structured`) on `node` in `cwd`; its id. */
export async function start(
  env: FleetEnv,
  node: string,
  cwd: string,
  mode: "terminal" | "structured",
): Promise<string> {
  const structured = mode === "structured" ? ["--structured"] : [];
  const out = await vessel(env, node, [
    "run",
    "claude",
    ...structured,
    "--detach",
    "--cwd",
    cwd,
  ]);
  const id = out.trim();
  if (id === "" || /\s/.test(id)) {
    throw new Error(`vessel run printed no session id: ${out}`);
  }
  return id;
}

/** Check `ready` until it holds or `ms` pass; `what` names the wait. */
export async function eventually(
  what: string,
  ready: () => Promise<boolean>,
  ms: number,
): Promise<void> {
  const deadline = Date.now() + ms;
  while (!(await ready())) {
    if (Date.now() > deadline) {
      throw new Error(`timed out waiting for ${what}`);
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
}

/** Send `prompt` to the structured session `id` and wait until its turn
 * has ended (its log says so). The prompt is refused until the agent is
 * ready, so it is sent again until it is taken. */
export async function promptAndWait(
  env: FleetEnv,
  node: string,
  id: string,
  prompt: string,
  ms: number,
): Promise<void> {
  let refusal = "";
  try {
    await eventually(
      `${id} to take a prompt`,
      async () =>
        vessel(env, node, ["send", id, prompt]).then(
          () => true,
          (error: unknown) => {
            refusal = String(error);
            return false;
          },
        ),
      ms,
    );
  } catch {
    throw new Error(`${id} did not take the prompt: ${refusal}`);
  }
  await eventually(
    `${id}'s turn to end`,
    async () => {
      const { stdout, stderr } = await vesselOutput(env, node, ["log", id]);
      return `${stdout}${stderr}`.includes("[turn ended");
    },
    ms,
  );
}

/** The ids of `node`'s sessions that are running. */
export async function running(env: FleetEnv, node: string): Promise<string[]> {
  const out = await vessel(env, node, ["sessions"]);
  return out
    .split("\n")
    .map((line) => line.split(/\s+/))
    .filter((fields) => fields.includes("running"))
    .map((fields) => fields[0] ?? "")
    .filter((id) => id !== "");
}

/** `work` over `items`, at most `width` at a time. */
export async function inParallel<T>(
  items: readonly T[],
  width: number,
  work: (item: T) => Promise<void>,
): Promise<void> {
  const queue = [...items];
  const lanes = Array.from({ length: width }, async () => {
    for (let item = queue.shift(); item !== undefined; item = queue.shift()) {
      await work(item);
    }
  });
  await Promise.all(lanes);
}

/** Stop the sessions `ids` on `node` (each with its process group). */
export async function stop(
  env: FleetEnv,
  node: string,
  ids: readonly string[],
): Promise<void> {
  await inParallel(ids, 16, async (id) => {
    await vessel(env, node, ["kill", id]);
  });
}

/** Processes and memory of a tree of processes. */
export interface Footprint {
  readonly processes: number;
  /** Resident memory of the roots alone (the node), in bytes. */
  readonly rootsBytes: number;
  /** Resident memory of the whole tree, each process's `VmRSS` summed:
   * pages the processes share (one binary's code) count once per
   * process, so this is more than the machine gives them. */
  readonly residentBytes: number;
}

/** What the processes `roots` and all their descendants hold now, from
 * /proc; `null` where there is none (not Linux). */
export function footprint(roots: readonly number[]): Footprint | null {
  let entries: string[];
  try {
    entries = readdirSync("/proc").filter((name) => /^\d+$/.test(name));
  } catch {
    return null;
  }
  const children = new Map<number, number[]>();
  for (const name of entries) {
    const parent = parentOf(name);
    if (parent !== null) {
      children.set(parent, [...(children.get(parent) ?? []), Number(name)]);
    }
  }
  const tree: number[] = [];
  const next = [...roots];
  for (let pid = next.pop(); pid !== undefined; pid = next.pop()) {
    tree.push(pid);
    next.push(...(children.get(pid) ?? []));
  }
  return {
    processes: tree.length,
    rootsBytes: roots.reduce((sum, pid) => sum + residentOf(pid), 0),
    residentBytes: tree.reduce((sum, pid) => sum + residentOf(pid), 0),
  };
}

/** The parent of process `pid` (`/proc/<pid>/stat`, the field after the
 * parenthesised name, which may itself hold spaces). */
function parentOf(pid: string): number | null {
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, "utf8");
    const fields = stat.slice(stat.lastIndexOf(")") + 2).split(" ");
    const parent = Number(fields[1]);
    return Number.isInteger(parent) ? parent : null;
  } catch {
    // Gone since the directory was listed.
    return null;
  }
}

function residentOf(pid: number): number {
  try {
    const status = readFileSync(`/proc/${String(pid)}/status`, "utf8");
    const kib = /^VmRSS:\s+(\d+) kB$/m.exec(status)?.[1];
    return kib === undefined ? 0 : Number(kib) * 1024;
  } catch {
    return 0;
  }
}

/** The fleet's process for node `name` (its command line names it). */
export function nodePid(env: FleetEnv, name: string): number | null {
  for (const pid of env.pids) {
    try {
      const args = readFileSync(`/proc/${String(pid)}/cmdline`, "utf8").split(
        "\0",
      );
      const at = args.indexOf("--name");
      if (at >= 0 && args[at + 1] === name) {
        return pid;
      }
    } catch {
      // Not Linux, or gone.
    }
  }
  return null;
}
