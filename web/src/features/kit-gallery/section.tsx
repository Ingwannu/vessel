// One section of the gallery: a heading, then rows of examples, each
// with a caption naming its state.

import type { ReactNode } from "react";

export function Section({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="kit-section" aria-label={title}>
      <h2>{title}</h2>
      {children}
    </section>
  );
}

/** A row of examples that share a caption. */
export function Specimens({
  caption,
  children,
}: {
  caption: string;
  children: ReactNode;
}) {
  return (
    <div className="kit-specimens">
      <span className="kit-caption">{caption}</span>
      <div className="kit-row">{children}</div>
    </div>
  );
}
