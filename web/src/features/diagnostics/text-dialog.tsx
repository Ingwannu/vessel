// A diagnostic's text in a dialog, as Paseo's diagnostic sheets show theirs
// (ADR 0111; `provider-diagnostic-sheet.tsx`, `app-diagnostic-sheet.tsx`):
// Copy and Refresh in the title row, "Running diagnostic..." until the
// first text, then the text in a monospace block. A failure's message is
// the text, as Paseo's sheet puts it in the block.

import "./diagnostics.css";

import type { ReactNode } from "react";

import { Dialog, IconButton, Spinner } from "../../ui/kit";
import { toast, toastCopied } from "../../ui/toasts";

/** Paseo's words (en.ts `settings.providers.diagnostic`,
 * `settings.diagnostics`). */
export const DIAGNOSTIC_WORDS = {
  running: "Running diagnostic...",
  copy: "Copy diagnostic",
  refresh: "Refresh diagnostic",
  refreshing: "Refreshing diagnostic",
  empty: "No diagnostic available",
  failed: "Failed to fetch diagnostic",
  copyFailed: "Failed to copy diagnostic",
} as const;

export function DiagnosticTextDialog({
  title,
  text,
  running,
  onRefresh,
  onClose,
  progress,
}: {
  title: string;
  /** The text so far (`null` before any). */
  text: string | null;
  running: boolean;
  onRefresh: () => void;
  onClose: () => void;
  /** Rows shown until the text exists (the app diagnostic's steps). */
  progress?: ReactNode;
}) {
  const copy = () => {
    if (text === null) return;
    navigator.clipboard.writeText(text).then(
      () => {
        toastCopied("diagnostic");
      },
      () => {
        toast(DIAGNOSTIC_WORDS.copyFailed, "error");
      },
    );
  };
  return (
    <Dialog
      title={title}
      onClose={onClose}
      size="lg"
      actions={
        <>
          <IconButton
            icon="copy"
            label={DIAGNOSTIC_WORDS.copy}
            disabled={text === null || running}
            onClick={copy}
          />
          <IconButton
            icon="refresh"
            label={
              running ? DIAGNOSTIC_WORDS.refreshing : DIAGNOSTIC_WORDS.refresh
            }
            disabled={running}
            aria-busy={running || undefined}
            onClick={onRefresh}
          />
        </>
      }
    >
      {/* Either the progress or the text, never both (Paseo's sheet:
          `diagnostic ? <ScrollableCodeSurface> : <SurfaceCard>`). */}
      {text === null ? (
        (progress ?? (
          <p className="diagnostic-wait">
            <Spinner />
            {DIAGNOSTIC_WORDS.running}
          </p>
        ))
      ) : (
        <pre className="diagnostic-text" data-testid="diagnostic-text">
          {text === "" ? DIAGNOSTIC_WORDS.empty : text}
        </pre>
      )}
    </Dialog>
  );
}
