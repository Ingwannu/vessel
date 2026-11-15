---
name: vessel-handoff
description: Transfer an explicitly requested task with a complete, truthful resume point and clear editing ownership.
user-invocable: true
---

# Vessel Handoff

Use the user's handoff request and current conversation. Read the vessel skill
and select the requested profile, or a discovered profile suited to the work.
Retain the user's exact task semantics: an investigation remains read-only;
a fix remains an implementation task; a refactor is not permission to rewrite.

Prepare a self-contained briefing for an agent with no prior context:

- The objective and why the task exists.
- Destination machine, working directory, branch and exact commit.
- Relevant file paths and their roles.
- Accepted decisions and reasons, including rejected approaches.
- Completed work, known defects and uncommitted changes with their ownership.
- Exact checks and their logs; distinguish failed, passed and unexecuted work.
- Acceptance criteria, user constraints and the concrete next action.

Preserve dirty work. Do not discard another writer's changes or assume a fresh
worktree contains uncommitted edits. If the user requests worktree isolation,
create the destination placement through the supported project/worktree tools
and explicitly account for any authorized work that must be carried over.

Launch one `[Handoff]` session in the selected placement with the briefing as
its first prompt. Establish which session may edit the checkout; the sender
must stop editing before the receiving worker begins. Do not let both writers
continue by accident.

Return the destination, session ID and placement to the user. Explain that the
receiver remains a child session until the user detaches it through the normal
user interface. Do not invoke internal detach operations or invent an
independent creation mode. The handoff ends after launching and reporting the
receiver; do not poll or wait for that worker to finish the transferred task.
