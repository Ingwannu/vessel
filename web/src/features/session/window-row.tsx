// One drawn row of the timeline's window (use-window.ts): measured while
// it is drawn, and its `<details>` opened or closed as the reader last
// left them when it is drawn again.

import { type ReactNode, useLayoutEffect, useRef } from "react";

import type { Rows } from "./use-window";

export function WindowRow({
  id,
  last,
  rows,
  children,
}: {
  /** The entry's key: the measurement's and the toggles' name. */
  id: string;
  /** Whether it is the timeline's last entry (its rail ends there). */
  last: boolean;
  rows: Rows;
  children: ReactNode;
}) {
  const element = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const row = element.current;
    if (row === null) {
      return undefined;
    }
    const states = rows.opened.get(id);
    if (states !== undefined) {
      row.querySelectorAll("details").forEach((details, index) => {
        details.open = states[index] ?? details.open;
      });
    }
    rows.observer.observe(row);
    return () => {
      rows.observer.unobserve(row);
    };
  }, [id, rows]);
  return (
    <div
      className="t-row"
      data-key={id}
      data-last={last ? "" : undefined}
      ref={element}
    >
      {children}
    </div>
  );
}
