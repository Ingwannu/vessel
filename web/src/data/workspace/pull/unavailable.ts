// Why there is no pull request to show, from the node's forge access: the
// native `view/src/workspace/pull/entry.rs`'s `Unavailable` and
// `Unavailable::of`, which the native pane's empty state words
// (`app/src/areas/pulls/pane.rs`, `empty`). Two of its sentences are
// Paseo's `getCreatePrUnavailableMessage` (`git/policy.ts`).
//
// [Decision Log]
// - Purpose: an empty pane says why ("this machine isn't signed in to
//   github.com"), not only that there is nothing.
// - Constraint: the node knows the forge (`ForgeAccess`); the page does
//   not ask GitHub itself. A newer node's reason reads as "unknown".
// - Choice: the reasons an access maps to, each keeping its host and its
//   time as values (the words put a host in a sentence the translator
//   owns, never the forge's own message).
// - Differences: the git menu's entry (`pull_entry`: "Create PR" or "View
//   PR", decided with the git status) is another feature and is not
//   ported, so its `NoCommits`, which no access maps to, is not here.

import type { ForgeAccess, ForgeHost } from "../../../wire/proto";

/** Why the node has no pull request to show, or would refuse one. */
export type Unavailable =
  /** On no branch. */
  | { readonly kind: "noBranch" }
  /** The branch's remote does not exist. */
  | { readonly kind: "noRemote" }
  /** No forge this node reads for `host`. */
  | { readonly kind: "notSupported"; readonly host: ForgeHost }
  /** The node is not signed in to `host`. */
  | { readonly kind: "signedOut"; readonly host: ForgeHost }
  /** `host` refused the node's sign-in. */
  | { readonly kind: "refused"; readonly host: ForgeHost }
  /** `host`'s rate limit holds until then (RFC 3339). */
  | {
      readonly kind: "rateLimited";
      readonly host: ForgeHost;
      readonly until: string;
    }
  /** `host` could not be reached; the node tries again then (RFC 3339). */
  | {
      readonly kind: "unreachable";
      readonly host: ForgeHost;
      readonly retryAt: string;
    }
  /** The node watches as many as it allows. */
  | { readonly kind: "limit" }
  /** The session ended. */
  | { readonly kind: "ended" }
  /** A reason this page does not know. */
  | { readonly kind: "unknown" };

/** Why `access` keeps a pull request from being read or opened. An access
 * that is no refusal (ready, on the base branch, outside a project) gives
 * "unknown", as the native rule does. */
export function unavailableOf(access: ForgeAccess): Unavailable {
  switch (access.access) {
    case "no_branch":
      return { kind: "noBranch" };
    case "no_remote":
      return { kind: "noRemote" };
    case "not_supported":
      return { kind: "notSupported", host: access.host };
    case "signed_out":
      return { kind: "signedOut", host: access.host };
    case "refused":
      return { kind: "refused", host: access.host };
    case "rate_limited":
      return { kind: "rateLimited", host: access.host, until: access.until };
    case "unreachable":
      return {
        kind: "unreachable",
        host: access.host,
        retryAt: access.retry_at,
      };
    case "limit":
      return { kind: "limit" };
    case "ended":
      return { kind: "ended" };
    case "ready":
    case "on_base":
    case "not_placed":
    case "unknown":
      return { kind: "unknown" };
  }
}
