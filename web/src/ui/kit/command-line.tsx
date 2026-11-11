// CommandLine (DESIGN.md §6.13): a command for the person to run
// elsewhere, in mono, with a button that copies it. GOAT's setup command
// (its preflight checklist's fix, its "Copy command"): a filled block,
// the command selectable, a ghost button at its right whose label says
// "Copied" for a moment after.
//
// The copy is the kit's own (the clipboard, then a word on the button),
// not the page's toast: a command is copied while a dialog is open, and a
// toast under a dialog's scrim is not seen.

import "./command-line.css";

import { useEffect, useState } from "react";

import { Button } from "./button";

type Copied = "idle" | "copied" | "refused";

export function CommandLine({
  command,
  label = "Copy command",
}: {
  /** The exact text to run. */
  command: string;
  /** The button's words before copying. */
  label?: string;
}) {
  const [copied, setCopied] = useState<Copied>("idle");
  useEffect(() => {
    if (copied === "idle") {
      return undefined;
    }
    const timer = setTimeout(() => {
      setCopied("idle");
    }, 1500);
    return () => {
      clearTimeout(timer);
    };
  }, [copied]);
  const word =
    copied === "copied"
      ? "Copied"
      : copied === "refused"
        ? "Copy failed"
        : label;
  return (
    <div className="command-line">
      <code className="command-line-text">{command}</code>
      <Button
        variant="ghost"
        size="xs"
        icon={copied === "copied" ? "check" : "copy"}
        aria-label={`${label}: ${command}`}
        onClick={() => {
          navigator.clipboard.writeText(command).then(
            () => {
              setCopied("copied");
            },
            () => {
              setCopied("refused");
            },
          );
        }}
      >
        <span aria-live="polite">{word}</span>
      </Button>
    </div>
  );
}
