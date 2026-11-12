// Repeated runs of a measurement, summarised (perf.ts): each quantity as
// its median over the runs with the smallest and the largest beside it,
// so a number before a change and one after it are compared with their
// spread in view. The runs themselves are kept too.

/** What a quantity is counted in. */
export type Unit = "ms" | "bytes" | "count" | "bytes/s" | "frames" | "px";

/** A quantity read from each run of type `R`. */
export interface Metric<R> {
  readonly name: string;
  readonly unit: Unit;
  readonly of: (run: R) => number;
}

/** A quantity over every run. */
export interface Row {
  readonly name: string;
  readonly unit: Unit;
  readonly median: number;
  readonly min: number;
  readonly max: number;
  readonly runs: readonly number[];
}

/** The middle of `values` (the mean of the two middle ones for an even
 * count); `NaN` for none. */
export function median(values: readonly number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  const upper = sorted[middle];
  if (upper === undefined) {
    return Number.NaN;
  }
  const lower = sorted.length % 2 === 0 ? sorted[middle - 1] : upper;
  return ((lower ?? upper) + upper) / 2;
}

/** Each of `metrics` over `runs`. */
export function rows<R>(
  metrics: readonly Metric<R>[],
  runs: readonly R[],
): Row[] {
  return metrics.map((metric) => {
    const values = runs.map(metric.of);
    return {
      name: metric.name,
      unit: metric.unit,
      median: median(values),
      min: Math.min(...values),
      max: Math.max(...values),
      runs: values,
    };
  });
}

/** A task the page's main thread ran for 50 ms or more (the Long Tasks
 * API's threshold), in the page's clock. */
export interface Task {
  readonly start: number;
  readonly duration: number;
}

/** Long tasks over a stretch of time. */
export interface LongTasks {
  readonly count: number;
  readonly totalMs: number;
  readonly longestMs: number;
}

/** The long tasks of `tasks` that began in `[from, to]`. */
export function longTasks(
  tasks: readonly Task[],
  from: number,
  to: number,
): LongTasks {
  const inside = tasks.filter((task) => task.start >= from && task.start <= to);
  return {
    count: inside.length,
    totalMs: inside.reduce((sum, task) => sum + task.duration, 0),
    longestMs: Math.max(0, ...inside.map((task) => task.duration)),
  };
}

/** The rows of a stretch's long tasks, `label` first: how many, their
 * total, the longest. */
export function taskMetrics<R>(
  label: string,
  of: (run: R) => LongTasks,
): Metric<R>[] {
  return [
    { name: `${label}: long tasks`, unit: "count", of: (run) => of(run).count },
    {
      name: `${label}: long tasks, total`,
      unit: "ms",
      of: (run) => of(run).totalMs,
    },
    {
      name: `${label}: long tasks, longest`,
      unit: "ms",
      of: (run) => of(run).longestMs,
    },
  ];
}

/** How smoothly frames came over a stretch of time. */
export interface Frames {
  /** Frames begun (animation-frame callbacks). */
  readonly drawn: number;
  /** Frames the stretch had room for at `referenceMs` each. */
  readonly expected: number;
  /** Gaps between consecutive frames longer than 1.5 × `referenceMs`:
   * at least one frame was missed in each. */
  readonly late: number;
  /** Frames missed: for each late gap, the frames that fit in it beyond
   * the one drawn. */
  readonly missed: number;
  readonly longestMs: number;
  /** The page's frame interval at rest, measured before the stretch. */
  readonly referenceMs: number;
}

/** `frames` (the start of each, in order) over `[from, to]`, against the
 * interval at rest `referenceMs`. */
export function frames(
  starts: readonly number[],
  from: number,
  to: number,
  referenceMs: number,
): Frames {
  const inside = starts.filter((start) => start >= from && start <= to);
  const gaps = inside.slice(1).map((start, index) => {
    const previous = inside[index] ?? start;
    return start - previous;
  });
  const late = gaps.filter((gap) => gap > 1.5 * referenceMs);
  return {
    drawn: inside.length,
    expected: Math.round((to - from) / referenceMs),
    late: late.length,
    missed: late.reduce(
      (sum, gap) => sum + Math.max(0, Math.round(gap / referenceMs) - 1),
      0,
    ),
    longestMs: Math.max(0, ...gaps),
    referenceMs,
  };
}
