// A worktree's setup as it runs (agent-orchestrator's script run panel):
// a 32px head with what it is doing and how long it has run, the output
// as it comes (the end kept, `data/workspace/setup-log.ts`), and, when it
// failed, why and what to do: run it again, start anyway, or stop here
// with the worktree kept.

import "./setup.css";

import { useEffect, useState } from "react";

import { type SetupLog, shownLines } from "../../data/workspace/setup-log";
import {
  setupFailureWords,
  setupWord,
} from "../../data/workspace/worktree-words";
import { Button, Icon, Spinner } from "../../ui/kit";
import type { SetupState } from "../../wire/proto";

/** Seconds since `iso`, as "12s" or "3m 4s". */
function elapsed(iso: string, now: number): string {
  const seconds = Math.max(0, Math.round((now - Date.parse(iso)) / 1000));
  return seconds < 60
    ? `${String(seconds)}s`
    : `${String(Math.floor(seconds / 60))}m ${String(seconds % 60)}s`;
}

export function SetupCard({
  branch,
  setup,
  log,
  commands,
  onRetry,
  onStart,
  onStop,
}: {
  /** The worktree's branch, to name it. */
  branch: string;
  setup: SetupState;
  log: SetupLog;
  /** The setup commands, to name the one that failed. */
  commands: readonly string[];
  onRetry?: () => void;
  onStart?: () => void;
  onStop?: () => void;
}) {
  const [now, setNow] = useState(() => Date.now());
  const running = setup.state === "running";
  useEffect(() => {
    if (!running) {
      return undefined;
    }
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => {
      window.clearInterval(timer);
    };
  }, [running]);
  const failed = setup.state === "failed";
  const lines = shownLines(log);
  return (
    <section
      className={failed ? "setup-card failed" : "setup-card"}
      aria-label="Worktree setup"
    >
      <header className="setup-head">
        {running ? (
          <Spinner size={14} />
        ) : (
          <Icon name={failed ? "circle-x" : "terminal"} size={14} />
        )}
        <span className="setup-title">
          {setupWord(setup)} · <span className="setup-branch">{branch}</span>
        </span>
        {setup.state === "running" ? (
          <span className="setup-time">{elapsed(setup.started_at, now)}</span>
        ) : null}
      </header>
      {lines.length === 0 ? null : (
        <pre className="setup-output" aria-live="polite">
          {log.cut ? "…\n" : ""}
          {lines.join("\n")}
        </pre>
      )}
      {setup.state === "failed" ? (
        <p className="setup-why" role="alert">
          {setupFailureWords(setup.failure, commands)} The worktree is kept; run
          setup again, start the session without it, or stop here.
        </p>
      ) : null}
      {onRetry === undefined &&
      onStart === undefined &&
      onStop === undefined ? null : (
        <footer className="setup-actions">
          {onStop === undefined ? null : (
            <Button size="xs" variant="ghost" onClick={onStop}>
              Stop here
            </Button>
          )}
          {onStart === undefined ? null : (
            <Button size="xs" onClick={onStart}>
              Start anyway
            </Button>
          )}
          {onRetry === undefined ? null : (
            <Button
              size="xs"
              variant="primary"
              icon="refresh"
              onClick={onRetry}
            >
              Run setup again
            </Button>
          )}
        </footer>
      )}
    </section>
  );
}
