// The account reports reader and the clock the usage cards share: Settings'
// plan usage (the machine's reports) and the session's hover card (its
// account's), so both read, refresh and tick the same way.
import { useEffect, useMemo, useState } from "react";

import { useFleet } from "../../context";
import {
  readAccountReports,
  refreshAccountReport,
  setAccountLookup,
  setSourceLookup,
} from "../../data/account-usage/calls";
import { QuotaInstant } from "../../data/account-usage/clock";
import {
  AccountReader,
  type AccountRead,
} from "../../data/account-usage/reader";
import { useStore } from "../../data/store";
import type { NodeKey } from "../../wire/bridge";
import type { SessionId } from "../../wire/proto";

/** Date.now() is always a safe integer; anything else is a broken clock. */
function instantNow(): QuotaInstant {
  const now = QuotaInstant.fromMilliseconds(Date.now());
  if (now === null) throw new Error("The device clock is out of range");
  return now;
}

/** Paseo's relative-time tick: once a minute while cards show times. */
function useNow(): QuotaInstant {
  const [now, setNow] = useState(instantNow);
  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(instantNow());
    }, 60_000);
    return () => {
      window.clearInterval(timer);
    };
  }, []);
  return now;
}

/** The minute tick, the browser language and its device clock format. */
export function useQuotaClock(): {
  now: QuotaInstant;
  locale: string;
  deviceClock: Intl.DateTimeFormat;
} {
  const now = useNow();
  const locale = navigator.language;
  const deviceClock = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, { hour: "numeric", minute: "2-digit" }),
    [locale],
  );
  return { now, locale, deviceClock };
}

/** `session` null reads the machine's reports; a session id reads only the
 * account that session runs under. */
export function useAccountReader(
  node: NodeKey,
  serves: boolean,
  session: SessionId | null,
): {
  reader: AccountReader;
  read: AccountRead;
} {
  const fleet = useFleet();
  const reader = useMemo(
    () =>
      new AccountReader({
        read: (key, refresh, scope) =>
          readAccountReports(fleet, key, refresh, scope),
        inspect: (key, id) => refreshAccountReport(fleet, key, id),
        lookup: (key, policy) => setAccountLookup(fleet, key, policy),
        source: (key, source, policy, version) =>
          setSourceLookup(fleet, key, source, policy, version),
      }),
    [fleet],
  );
  const read = useStore(reader.store, (value) => value);
  useEffect(() => {
    if (!serves) reader.stop();
    else if (session === null) reader.open(node);
    else reader.open(node, { kind: "session", session });
    return () => {
      reader.stop();
    };
  }, [reader, node, serves, session]);
  return { reader, read };
}
