// Paseo's usage read error (packages/app/src/usage/usage-section.tsx:72-82),
// apart from plan-usage.tsx so that file stays inside the 300-line budget and
// the view is rendered on its own in its test (TRACKER 820).

import { Alert, Button } from "../../ui/kit";

/** Paseo's UsageBody error: an Alert in the flow, what failed over the
 * node's reason, its Retry reading the machine's reports again (TRACKER
 * 820). */
export function UsageReadFailed({
  message,
  onRetry,
}: {
  message: string;
  onRetry: () => void;
}) {
  return (
    <Alert
      size="sm"
      variant="error"
      title="Unable to load usage"
      description={message}
    >
      <Button size="sm" variant="outline" onClick={onRetry}>
        Try again
      </Button>
    </Alert>
  );
}
