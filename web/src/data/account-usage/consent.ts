// A source's consent row (ADR 0042): what an explicit refresh may read and
// where it is sent, and the one action the Admin can take. Same rules as
// the native app (spikes/native/app/src/areas/usage/source_controls.rs):
// a source this machine cannot read has no row (Paseo lists only installed
// sources), and the three original stores follow the machine-wide switch.
import type {
  ExternalLookup,
  UsageCredentialStore,
  UsageRecipient,
  UsageReports,
  UsageSourceAvailability,
  UsageSourceDescriptor,
} from "../../wire/proto";

const STORE: Record<UsageCredentialStore, string> = {
  claude_code: "the configured Claude Code sign-in",
  codex: "the configured Codex sign-in",
  open_code: "OpenCode’s saved OpenAI sign-in",
  git_hub_copilot:
    "Copilot/GitHub environment credentials or the GitHub CLI sign-in",
  cursor:
    "Cursor environment credentials, its read-only desktop sign-in database, or its headless sign-in",
  grok: "Grok environment credentials or the Grok CLI sign-in",
  kimi: "Kimi environment credentials or saved Kimi Code/Kimi credentials",
  mini_max:
    "MiniMax environment credentials or saved MiniMax configuration and sign-in",
  open_code_go: "OpenCode’s saved Go API key",
  zai: "the Z.ai/GLM environment API key",
};

const RECIPIENT: Record<UsageRecipient, string> = {
  anthropic: "Anthropic",
  open_ai: "OpenAI’s ChatGPT usage service",
  git_hub: "GitHub",
  cursor: "Cursor",
  xai: "xAI’s Grok CLI service",
  kimi: "Kimi’s coding service",
  mini_max_global_or_china:
    "MiniMax’s global or China service, as configured locally",
  open_code: "OpenCode’s Go usage service",
  zai: "Z.ai",
};

const AVAILABILITY: Record<UsageSourceAvailability, string> = {
  unsupported: "This source is not available on this machine.",
  global_off:
    "Account limit lookup is off. Reported tokens and cost remain available.",
  source_off: "Source lookup is off.",
  disclosure_needs_consent:
    "Review this source’s updated sign-in access before enabling it.",
  enabled: "Enabled",
};

export interface ConsentRow {
  source: UsageSourceDescriptor;
  disclosure: string;
  state: string;
  /** What the button sets. */
  next: ExternalLookup;
  action: string;
}

export function disclosureText(source: UsageSourceDescriptor): string {
  return `An explicit refresh may read ${STORE[source.disclosure.store]} on this machine and send that sign-in to ${RECIPIENT[source.disclosure.recipient]}. Enabling access does not fetch usage.`;
}

/** The rows to show, in the node's order. */
export function consentRows(reports: UsageReports): ConsentRow[] {
  return reports.sources.flatMap((source) => {
    if (source.availability === "unsupported") return [];
    const consent = source.consent;
    if (consent.kind === "inherited_global") return [];
    const next: ExternalLookup =
      consent.kind === "granted" &&
      consent.version === source.disclosure.version
        ? "off"
        : "on";
    return [
      {
        source,
        disclosure: disclosureText(source),
        state: AVAILABILITY[source.availability],
        next,
        action:
          next === "on" ? "Enable source access" : "Disable source access",
      },
    ];
  });
}
