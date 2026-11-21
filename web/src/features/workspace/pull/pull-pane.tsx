// The Pull request tab's pane (TRACKER 430, 431, 434 on the page): Paseo's
// `pull-request-panel/pane.tsx` (pinned 0.15.4) over the node's reads
// (`use-pull.ts`): a toolbar (Open on GitHub, Refresh), the header (state,
// title, number, repository and branches), the checks and the activity;
// loading, failed (Retry) and unavailable or empty states.
//
// [Decision Log]
// - Purpose: the session's pull request in the workspace's tabs, as the
//   native app's Pull request tab, without leaving for the forge.
// - Constraint: the page's workspace tabs are switch-based and mounted
//   only while selected (workspace.tsx); the pane re-reads on open.
// - Choice: the anchor session's pull request; "Add to chat" writes into
//   that session's draft and switches to its chat tab (`openChat`).
// - Differences: no auto-open (the tab is offered in the launcher while
//   the session has a pull request); Add to chat writes text, not a pill.

import "./pull-pane.css";

import type { About } from "../../../data/workspace/pull/attach";
import { brand } from "../../../data/workspace/pull/card-menu";
import { phaseOf } from "../../../data/workspace/pull/facts";
import {
  phaseWords,
  unavailableWords,
} from "../../../data/workspace/pull/facts-words";
import { unavailableOf } from "../../../data/workspace/pull/unavailable";
import { pullWords } from "../../../data/workspace/pull/words";
import { Button, EmptyState, IconButton, Skeleton } from "../../../ui/kit";
import type { NodeKey } from "../../../wire/bridge";
import type { PullRequest, SessionInfo } from "../../../wire/proto";
import { PullActivity } from "./pull-activity";
import { PullChecks } from "./pull-checks";
import { useAddToDraft, usePull } from "./use-pull";

export function PullPane({
  node,
  session,
  served,
  openChat,
}: {
  node: NodeKey;
  session: SessionInfo;
  /** The machine answers pr.timeline and pr.check (protocol 39). */
  served: boolean;
  openChat: () => void;
}) {
  if (!served) {
    // A tab kept from before, on a machine that cannot answer: say so,
    // and ask it nothing (the launcher offers no such tab).
    return <PullTooOld language={navigator.language} />;
  }
  return <PullReads node={node} session={session} openChat={openChat} />;
}

/** The pane on a machine before protocol 39 (the node reviewer's R1). */
export function PullTooOld({ language }: { language: string }) {
  const words = pullWords(language);
  return (
    <div className="pull-pane">
      <EmptyState title={words.tooOldTitle} icon="info">
        {words.tooOldDescription}
      </EmptyState>
    </div>
  );
}

function PullReads({
  node,
  session,
  openChat,
}: {
  node: NodeKey;
  session: SessionInfo;
  openChat: () => void;
}) {
  const words = pullWords(navigator.language);
  const reads = usePull(node, session.id);
  const write = useAddToDraft(session.id);
  const add = (texts: readonly string[]) => {
    if (texts.length === 0) return;
    write(texts);
    openChat();
  };
  const { view, timeline } = reads;
  if (view === null || timeline === null) {
    return (
      <div className="pull-pane">
        <Skeleton />
      </div>
    );
  }
  if (!view.ok) {
    return (
      <div className="pull-pane">
        <EmptyState
          title={words.paneFailed}
          icon="warn"
          action={
            <Button
              variant="secondary"
              size="sm"
              busy={reads.refreshing}
              onClick={reads.refresh}
            >
              {words.paneRetry}
            </Button>
          }
        >
          {view.problem.message}
        </EmptyState>
      </div>
    );
  }
  const pull = view.value.pull_request ?? null;
  if (pull === null) {
    const why = unavailableOf(view.value.access);
    return (
      <div className="pull-pane">
        <EmptyState title={words.emptyTitle} icon="git-pull-request">
          {why.kind === "unknown"
            ? words.emptyDescription
            : unavailableWords(why, words)}
        </EmptyState>
      </div>
    );
  }
  const now = reads.readAt;
  const summary = pull.summary;
  const about: About | null =
    summary.url === ""
      ? null
      : { number: summary.number, title: summary.title, url: summary.url, now };
  const forge = brand(summary.repo);
  const items = timeline.ok ? timeline.value.items : [];
  return (
    <div className="pull-pane">
      <Toolbar
        pull={pull}
        forge={forge}
        refreshing={reads.refreshing}
        refresh={reads.refresh}
        words={words}
      />
      <Header pull={pull} words={words} />
      <PullChecks
        checks={pull.checks}
        words={words}
        now={now}
        about={about}
        report={reads.report}
        add={add}
      />
      <PullActivity
        items={items}
        words={words}
        now={now}
        forge={forge}
        about={about}
        add={add}
      />
    </div>
  );
}

function Toolbar({
  pull,
  forge,
  refreshing,
  refresh,
  words,
}: {
  pull: PullRequest;
  forge: string;
  refreshing: boolean;
  refresh: () => void;
  words: ReturnType<typeof pullWords>;
}) {
  return (
    <div className="pull-toolbar">
      {pull.summary.url === "" ? null : (
        <a
          className="pull-view"
          href={pull.summary.url}
          target="_blank"
          rel="noreferrer"
          aria-label={words.openOn(forge)}
        >
          {words.paneView}
        </a>
      )}
      <IconButton
        label={refreshing ? words.refreshing : words.refreshState(forge)}
        icon="refresh"
        size="xs"
        disabled={refreshing}
        onClick={refresh}
      />
    </div>
  );
}

function Header({
  pull,
  words,
}: {
  pull: PullRequest;
  words: ReturnType<typeof pullWords>;
}) {
  const summary = pull.summary;
  const phase = phaseOf(summary);
  return (
    <header className="pull-header">
      <span className={`pull-state tone-${phase}`}>
        {phaseWords(phase, words)}
      </span>
      <h2 className="pull-title">
        {summary.title}{" "}
        <span className="pull-muted">{`#${String(summary.number)}`}</span>
      </h2>
      <p className="pull-muted pull-where">
        {`${summary.repo.path} · ${pull.head_branch} → ${pull.base}`}
      </p>
    </header>
  );
}
