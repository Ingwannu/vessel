// Send on the new-session page while the agent's options are still being
// read (the new chat, 2026-11-13): it waits for them, at most
// OPTIONS_WAIT_MS, as agent-orchestrator's send waits up to 10 s for its
// page to load. Started before, the remembered and chosen choices could
// not be checked against what the agent offers (a choice it does not
// offer fails the session, ADR 0022), so the session started on the
// agent's own and the kept ones were replaced by none.

import { useEffect, useEffectEvent, useState } from "react";

/** How long Send waits for the agent's options. */
export const OPTIONS_WAIT_MS = 10_000;

/** `wait()` holds `start` until `loading` is over, or OPTIONS_WAIT_MS
 * passed; `waiting` says it is held. */
export function useOptionsWait(
  loading: boolean,
  start: () => void,
): { waiting: boolean; wait: () => void } {
  const [waiting, setWaiting] = useState(false);
  const go = useEffectEvent(() => {
    setWaiting(false);
    start();
  });
  useEffect(() => {
    if (!waiting) {
      return undefined;
    }
    const timer = window.setTimeout(go, loading ? OPTIONS_WAIT_MS : 0);
    return () => {
      window.clearTimeout(timer);
    };
  }, [waiting, loading]);
  return {
    waiting,
    wait: () => {
      setWaiting(true);
    },
  };
}
