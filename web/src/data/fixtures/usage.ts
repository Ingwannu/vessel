// Node wire shape for the testkit's ACP usage report; no real login/account.
import type { UsageReport } from "../../wire/proto";

export function usageReport(): UsageReport {
  return {
    at: "2026-11-14T12:00:00Z",
    accounting_started: "2026-11-14T00:00:00Z",
    retained_from: "2026-08-16T12:00:00Z",
    incomplete: false,
    cost_visibility: "permitted",
    current_costs: [],
    totals: {
      tokens: {
        total: 30,
        input: 20,
        output: 10,
        thought: 0,
        cached_read: 12,
        cached_write: 0,
      },
      token_accounting: "reported",
      costs: [{ amount: 0.5, unit: "USD" }],
      reported: 1,
      unreported: 0,
      unaccounted_costs: 0,
      optional_reports: { thought: 1, cached_read: 1, cached_write: 1 },
    },
    agents: [],
    samples: [],
    next: null,
  };
}
