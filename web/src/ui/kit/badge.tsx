// Badge (DESIGN.md §6.4): a quiet outlined word. State is a 6px dot of its
// colour in front of the word; only danger also colours the word, so a
// row of badges does not turn into a row of colours.

import "./badge.css";

import type { ComponentProps, ReactNode } from "react";

/** The colours state has (DESIGN.md §4.1): ok is the ready lane's green,
 * warn the review lane's amber, danger the danger red, interrupt the
 * tertiary grey. `neutral` has no dot. */
export type Tone = "neutral" | "ok" | "warn" | "danger" | "interrupt";

/** `word` is the default; `accent` is ink on the faintest wash (the
 * fleet's home, a book's default); `mono` is a machine's word (a
 * transport, a scope) in the mono face. */
export type BadgeLook = "word" | "accent" | "mono";

function toneClass(tone: Tone): string {
  switch (tone) {
    case "neutral":
      return "";
    case "ok":
      return " tone-ok";
    case "warn":
      return " tone-warn";
    case "danger":
      return " tone-danger";
    case "interrupt":
      return " tone-interrupt";
  }
}

function lookClass(look: BadgeLook): string {
  switch (look) {
    case "word":
      return "";
    case "accent":
      return " accent";
    case "mono":
      return " mono";
  }
}

/** The span's own props pass through, so a Tooltip can name a badge. */
export function Badge({
  tone = "neutral",
  look = "word",
  children,
  ...rest
}: {
  tone?: Tone;
  look?: BadgeLook;
  children: ReactNode;
} & Omit<ComponentProps<"span">, "className" | "style" | "children">) {
  return (
    <span {...rest} className={`badge${lookClass(look)}${toneClass(tone)}`}>
      {children}
    </span>
  );
}
