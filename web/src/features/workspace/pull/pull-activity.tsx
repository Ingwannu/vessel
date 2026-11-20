// The Pull request pane's activity (TRACKER 430 on the page): Paseo's
// activity list in `pull-request-panel/pane.tsx` (pinned 0.15.4):
// comments, reviews with the threads started in them, and threads, newest
// last; the rules are `data/workspace/pull/timeline.ts`.
//
// [Decision Log]
// - Purpose: what reviewers said, folded where Paseo folds it (resolved
//   and outdated threads), each card with Paseo's kebab (Add to chat,
//   Copy, Open on GitHub) and "Add all to chat" for the lot.
// - Choice: a card's header is the fold's button; the kebab is the kit's
//   Menu (shown on hover with a fine pointer, always on touch); Add to
//   chat writes Paseo's context text into the session's draft.
// - Differences: bodies are plain text kept as written (Paseo renders
//   markdown); Add to chat writes text, not a pill (TRACKER 833).

import { type ReactNode, useState } from "react";

import {
  activityAttachment,
  threadAttachment,
  type About,
} from "../../../data/workspace/pull/attach";
import {
  copied,
  itemActs,
  threadActs,
} from "../../../data/workspace/pull/card-menu";
import {
  entries,
  entryFoldedAtFirst,
  entryKey,
  itemAge,
  threadPath,
  type Entry,
  type Thread,
} from "../../../data/workspace/pull/timeline";
import {
  ageWords,
  verbWords,
} from "../../../data/workspace/pull/timeline-words";
import type { PullWords } from "../../../data/workspace/pull/words";
import { Button, Icon } from "../../../ui/kit";
import type { TimelineItem } from "../../../wire/proto";
import { Kebab } from "./pull-kebab";

export interface ActivityProps {
  items: readonly TimelineItem[];
  words: PullWords;
  now: number;
  /** The forge's name for "Open on …". */
  forge: string;
  about: About | null;
  add: (texts: readonly string[]) => void;
}

export function PullActivity(props: ActivityProps) {
  const { items, words, about, add } = props;
  const listed = entries(items);
  const all = about === null ? [] : attachAll(about, listed);
  return (
    <section className="pull-section" aria-label={words.activity}>
      <div className="pull-section-head">
        <span className="pull-section-title">{words.activity}</span>
        {all.length === 0 ? null : (
          <Button
            variant="ghost"
            size="xs"
            icon="message"
            onClick={() => {
              add(all);
            }}
          >
            {words.addAllToChat}
          </Button>
        )}
      </div>
      {listed.length === 0 ? (
        <p className="pull-muted">{words.noActivity}</p>
      ) : (
        listed.map((entry) => (
          <EntryCard key={entryKey(entry)} entry={entry} {...props} />
        ))
      )}
    </section>
  );
}

function attachAll(about: About, listed: readonly Entry[]): string[] {
  return listed
    .flatMap((entry) => {
      switch (entry.kind) {
        case "single":
          return [activityAttachment(about, entry.item)?.text ?? null];
        case "thread":
          return [threadAttachment(about, entry.thread)?.text ?? null];
        case "review":
          return [
            activityAttachment(about, entry.review)?.text ?? null,
            ...entry.threads.map(
              (thread) => threadAttachment(about, thread)?.text ?? null,
            ),
          ];
      }
    })
    .filter((text): text is string => text !== null);
}

function EntryCard({ entry, ...props }: ActivityProps & { entry: Entry }) {
  switch (entry.kind) {
    case "single":
      return <ItemCard item={entry.item} {...props} />;
    case "thread":
      return <ThreadCard thread={entry.thread} {...props} />;
    case "review":
      return (
        <ItemCard item={entry.review} {...props}>
          {entry.threads.map((thread) => (
            <ThreadCard key={thread.place.thread} thread={thread} {...props} />
          ))}
        </ItemCard>
      );
  }
}

function ItemCard({
  item,
  children,
  ...props
}: ActivityProps & { item: TimelineItem; children?: ReactNode }) {
  const { words, now, about, add } = props;
  const [open, setOpen] = useState(true);
  const text = item.body.trim();
  const acts = itemActs(item, about !== null);
  return (
    <article className="pull-card">
      <div className="pull-card-head">
        <button
          type="button"
          className="pull-card-fold"
          aria-expanded={open}
          onClick={() => {
            setOpen(!open);
          }}
        >
          <span className="pull-card-author">
            {item.author ?? words.unknownAuthor}
          </span>
          <span className="pull-muted">{verbWords(item, words)}</span>
          <span className="pull-muted">
            {ageWords(itemAge(item, now), words)}
          </span>
        </button>
        <Kebab
          acts={acts}
          label={words.commentActions}
          words={words}
          forge={props.forge}
          url={item.url ?? null}
          copyText={copied(item)}
          onAdd={() => {
            const made =
              about === null ? null : activityAttachment(about, item);
            if (made !== null) add([made.text]);
          }}
        />
      </div>
      {open && text !== "" ? (
        <p className="pull-card-body">{item.body}</p>
      ) : null}
      {open ? children : null}
    </article>
  );
}

function ThreadCard({ thread, ...props }: ActivityProps & { thread: Thread }) {
  const { words, now, about, add } = props;
  const [open, setOpen] = useState(
    !entryFoldedAtFirst({ kind: "thread", thread }),
  );
  const place = threadPath(thread.place);
  const first = thread.comments[0];
  const badge = thread.place.resolved
    ? words.threadResolved
    : thread.place.outdated
      ? words.threadOutdated
      : null;
  return (
    <article className="pull-card pull-thread">
      <div className="pull-card-head">
        <button
          type="button"
          className="pull-card-fold"
          aria-expanded={open}
          onClick={() => {
            setOpen(!open);
          }}
        >
          <Icon name={open ? "chevron-down" : "chevron"} size={12} />
          <span className="pull-thread-place">
            {place ?? words.threadDiscussion}
          </span>
          {badge === null ? null : <span className="pull-badge">{badge}</span>}
        </button>
        <Kebab
          acts={threadActs(first)}
          label={words.threadActions}
          words={words}
          forge={props.forge}
          url={first.url ?? null}
          copyText={null}
          onAdd={() => {
            const made =
              about === null ? null : threadAttachment(about, thread);
            if (made !== null) add([made.text]);
          }}
        />
      </div>
      {open
        ? thread.comments.map((comment) => (
            <div key={comment.id} className="pull-thread-comment">
              <div className="pull-card-meta">
                <span className="pull-card-author">
                  {comment.author ?? words.unknownAuthor}
                </span>
                <span className="pull-muted">
                  {ageWords(itemAge(comment, now), words)}
                </span>
              </div>
              <p className="pull-card-body">{comment.body}</p>
            </div>
          ))
        : null}
    </article>
  );
}
