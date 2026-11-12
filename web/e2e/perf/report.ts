// The measurements' results (perf.ts): printed as each one finishes and
// kept as JSON in `VESSEL_PERF_OUT` (default artifacts/perf.json, which
// is untracked), one file per run of `npm run perf`.
//
// Each measurement is its own test, and a test may run alone (`npm run
// perf -- -g timeline`), so a test adds its section to the file instead
// of a run writing it once at the end. A section from an earlier run is
// not kept beside a new one: the file records which fleet it came from,
// and a new fleet (a new `npm run perf`) starts a new file.
//
// Nothing that names the machine or a person goes in: no host name, no
// address, no path, no launch URL. The machine is described by what
// bears on the numbers: its processor count and model, memory, and how
// loaded it was while each measurement ran.

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { cpus, loadavg, totalmem } from "node:os";
import { dirname, join } from "node:path";

import { cargoProfile, type FleetEnv } from "../fleet";
import type { Row } from "./stats";

const OUT =
  process.env["VESSEL_PERF_OUT"] ??
  join(import.meta.dirname, "..", "..", "artifacts", "perf.json");

/** Runs per measurement: `VESSEL_PERF_RUNS`, else `fallback`. */
export function runs(fallback: number): number {
  const given = Number(process.env["VESSEL_PERF_RUNS"] ?? "");
  return Number.isInteger(given) && given > 0 ? given : fallback;
}

/** The rows of one condition or size measured. */
export interface Table {
  readonly name: string;
  readonly rows: readonly Row[];
}

/** One measurement's results. */
export interface Section {
  readonly title: string;
  /** How it was measured, in words: the definitions behind the rows. */
  readonly method: readonly string[];
  readonly tables: readonly Table[];
  /** What was seen beside the numbers (counts checked, limits met). */
  readonly notes: readonly string[];
  /** The machine's one-minute load average before and after it ran. */
  readonly load: { readonly before: number; readonly after: number };
}

/** The one-minute load average now: a shared machine's noise. */
export function loadNow(): number {
  return loadavg()[0] ?? Number.NaN;
}

/** Print `section` and add it, under `key`, to the run's file. */
export function record(
  env: FleetEnv,
  browser: string,
  key: string,
  section: Section,
): void {
  print(section);
  const run = createHash("sha256").update(env.root).digest("hex").slice(0, 12);
  const earlier = previousSections(run);
  const machine = cpus();
  const file = {
    run,
    written: new Date().toISOString(),
    binaries: cargoProfile(),
    browser,
    machine: {
      platform: process.platform,
      arch: process.arch,
      processors: machine.length,
      model: machine[0]?.model ?? "",
      memoryBytes: totalmem(),
    },
    sections: { ...earlier, [key]: section },
  };
  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, `${JSON.stringify(file, null, 2)}\n`);
  process.stdout.write(`perf: ${key} written to ${OUT}\n`);
}

/** The sections already in the file, if it is this run's. */
function previousSections(run: string): Record<string, unknown> {
  if (!existsSync(OUT)) {
    return {};
  }
  const parsed: unknown = JSON.parse(readFileSync(OUT, "utf8"));
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("run" in parsed) ||
    parsed.run !== run ||
    !("sections" in parsed) ||
    typeof parsed.sections !== "object" ||
    parsed.sections === null
  ) {
    return {};
  }
  const sections: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(parsed.sections)) {
    sections[key] = value;
  }
  return sections;
}

function print(section: Section): void {
  const lines = [
    "",
    `== ${section.title} (load ${section.load.before.toFixed(1)} → ${section.load.after.toFixed(1)})`,
    ...section.method.map((line) => `   ${line}`),
  ];
  for (const table of section.tables) {
    lines.push(`-- ${table.name}`);
    const width = Math.max(...table.rows.map((row) => row.name.length));
    for (const row of table.rows) {
      lines.push(
        `   ${row.name.padEnd(width)}  ${show(row.median, row.unit)} (${show(row.min, row.unit)}–${show(row.max, row.unit)}) ${row.unit}, n=${String(row.runs.length)}`,
      );
    }
  }
  lines.push(...section.notes.map((note) => `   note: ${note}`), "");
  process.stdout.write(lines.join("\n"));
}

/** A value as the table shows it: whole bytes and counts, tenths of a
 * millisecond or a pixel. */
function show(value: number, unit: Row["unit"]): string {
  switch (unit) {
    case "ms":
    case "px":
      return value.toFixed(1);
    case "bytes":
    case "count":
    case "frames":
    case "bytes/s":
      return Math.round(value).toLocaleString("en-US");
  }
}
