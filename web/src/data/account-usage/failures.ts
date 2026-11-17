// Why an account report has no current values, in the page's words.
// Copied from the native usage words (view/locales/en/usage.ftl); a change
// there is made here too. Source and vendor messages never reach the page.
import type {
  LimitFailure,
  UsageFetchFailure,
  UsageProblem,
} from "../../wire/proto";

const LOGIN: Record<LimitFailure, string> = {
  no_login: "No saved sign-in. Sign in through the agent first.",
  expired: "The saved sign-in expired. Refresh it through the agent.",
  rejected:
    "The vendor rejected the saved sign-in. Sign in through the agent again.",
  unreachable: "The vendor could not be reached.",
  changed_response: "The vendor response could not be read safely.",
  unsupported: "This agent does not support account limit lookup.",
  unsafe_login_store: "The saved sign-in could not be read safely.",
  different_route: "This session uses a different provider route.",
  unassociated: "This session’s original saved account is no longer available.",
  rate_limited: "The vendor requested a cooldown before checking again.",
  busy: "A usage request is already in progress. Try again shortly.",
};

export const FETCH: Record<UsageFetchFailure, string> = {
  timed_out: "The usage request timed out. Try again.",
  unreachable:
    "The source could not reach its fixed usage service. Try again later.",
  changed_response: "The vendor response could not be read safely.",
  source_removed: "This source is not available on this machine.",
  busy: "A usage request is already in progress. Try again shortly.",
  helper_unavailable: "This source is not available on this machine.",
};

export function problemText(problem: UsageProblem): string {
  if (problem.kind === "login") return LOGIN[problem.reason];
  switch (problem.reason) {
    case "no_active_plan":
      return "No active coding plan was reported.";
    case "no_allowance":
      return "This login has no reported allowance.";
    case "inactive":
      return "The reported subscription is inactive.";
  }
}
