// What "Add to chat" hands the composer for a failed check: Paseo's
// `pull-request-panel/context-attachment.ts` (pinned 0.15.4) check text,
// with what `pr.check` read of it when it could (its output, annotations,
// failed jobs and their log tails; ADR 0088 part 4). Mirrors the check
// half of the native `view/src/workspace/pull/attach.rs` (its Decision
// Log is attach.ts's): English for the agent, line for line.

import type {
  AnnotationLevel,
  CheckReport,
  CheckRun,
  CheckState,
} from "../../../wire/proto";
import { aboutLines, type About, type Attachment } from "./attach";

/** A check state as Paseo's wire names it. */
function checkStatus(state: CheckState): string {
  switch (state) {
    case "passed":
      return "success";
    case "failed":
      return "failure";
    case "skipped":
      return "skipped";
    case "cancelled":
      return "cancelled";
    case "pending":
    case "unknown":
      return "pending";
  }
}

/** An annotation's level after its lines; nothing for one not known. */
function levelWord(level: AnnotationLevel): string {
  switch (level) {
    case "notice":
      return " notice";
    case "warning":
      return " warning";
    case "failure":
      return " failure";
    case "unknown":
      return "";
  }
}

/** Paseo's `canAddPullRequestCheckLogsToChat`: a failed check. */
export function canAddCheck(run: CheckRun): boolean {
  return run.state === "failed";
}

/** A failed check, with what `pr.check` read of it when it could
 * (`report`, null when the read failed or has not run). */
export function checkAttachment(
  about: About,
  run: CheckRun,
  report: CheckReport | null,
): Attachment {
  const checkUrl = run.url ?? about.url;
  const lines = aboutLines(about, "GitHub pull request check");
  lines.push(`Check: ${run.name}`, `Status: ${checkStatus(run.state)}`);
  // A report still pending has no conclusion; one the node does not know
  // reads as pending, as the native rule's `state != Pending` lets it.
  if (report !== null && report.state !== "pending") {
    lines.push(`Conclusion: ${checkStatus(report.state)}`);
  }
  lines.push(`Check URL: ${checkUrl}`);
  const details = report?.details_url ?? report?.url ?? null;
  if (details !== null) {
    lines.push(`Details URL: ${details}`);
  }
  if (report !== null) {
    lines.push(...reportLines(report));
  }
  const id =
    run.run === undefined || run.run === null
      ? `${about.number}:check:${run.name}`
      : `${about.number}:check-run:${run.run}`;
  return {
    kind: "check",
    id,
    title: run.name,
    text: lines.join("\n"),
    url: details ?? checkUrl,
  };
}

/** The output, the annotations, the failed jobs and their logs. */
function reportLines(report: CheckReport): string[] {
  const lines: string[] = [];
  const output = report.output ?? null;
  if (output !== null) {
    // Paseo skips an empty one as an absent one (truthy checks).
    if (output.title != null && output.title !== "") {
      lines.push(`Output title: ${output.title}`);
    }
    if (output.summary != null && output.summary !== "") {
      lines.push(`Output summary: ${output.summary}`);
    }
    if (output.text != null && output.text !== "") {
      lines.push("Output text:", output.text);
    }
  }
  if (report.annotations.length > 0) {
    lines.push("", "Annotations:");
    for (const note of report.annotations) {
      const message = note.message === "" ? "" : `: ${note.message}`;
      lines.push(
        `- ${note.path}:${note.start_line}-${note.end_line}${levelWord(note.level)}${message}`,
      );
    }
  }
  if (report.failed_jobs.length > 0) {
    lines.push("", "Failed jobs:");
    for (const job of report.failed_jobs) {
      lines.push(`- ${job.name}: ${checkStatus(job.state)}`);
      if (job.url !== undefined && job.url !== null) {
        lines.push(`  ${job.url}`);
      }
      const log = job.log ?? null;
      if (log !== null) {
        lines.push(
          "  ```",
          ...log.text.split("\n").map((line) => `  ${line}`),
          "  ```",
        );
        if (log.cut) {
          lines.push("  Log tail truncated to the latest capped lines.");
        }
      }
    }
  }
  if (report.truncated) {
    lines.push(
      "",
      "Note: Check details were truncated by GitHub/API or local caps.",
    );
  }
  return lines;
}
