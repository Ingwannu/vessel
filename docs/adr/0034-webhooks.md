# ADR 0034: Webhooks that start a session

Status: accepted, 2026-11-12 (milestone 11, track D,
`docs/plans/backend-m11.md`, "Track D: contract", "Webhooks"; the wire is
protocol 11's `HookDefinition`, `HookInfo`, `HookSecret` and
`HookDelivery` in `crates/vessel-proto/src/automation/hook.rs`, kept by
schema 13). The decision below is the design. The listener is not built:
"Where it stands" says why and what remains.

## Context

GOAT starts a task from an HTTP request (a token in the path, an
optional HMAC over the body without a signed time, filters, dedup by a
template, sixty a minute, a delivery log). agent-orchestrator and Paseo
have no inbound webhooks. The owner asked for Vessel to cover what the
three do, and webhooks are how a CI run, a monitor or a form starts an
agent without a person.

A webhook is the first door into a node that is not a client of it: the
sender holds no device key and no node token, and what it sends is read
by an agent that may be able to run commands. So:

1. How does the node know a request is from the hook's sender, once, and
   recent?
2. What does a request cost the node before it is known to be good, and
   what bounds the rest?
3. Where is the secret, and who ever sees it?
4. What does the body become for the agent, and what may that agent do?
5. Is any of it something the node cannot make safe?

## Options

**The signature**

1. A token in the URL (GOAT's default): appears in proxies' and senders'
   logs; anyone who sees one URL can send forever; nothing dates a
   request, so a captured one can be sent again.
2. Each sender's own scheme (GitHub's `X-Hub-Signature-256`, Stripe's,
   Slack's): three schemes, three sets of rules on time and replay, the
   node trusting each provider's quirks.
3. Standard Webhooks (an open specification, used by Svix, Supabase and
   others; libraries in many languages): `webhook-id`,
   `webhook-timestamp`, `webhook-signature: v1,<base64>` = HMAC-SHA256
   over `<id>.<timestamp>.<body>` under a `whsec_` secret. The id and the
   time are signed: a captured request cannot be sent again under a new
   id, nor later than the tolerance.

**Where the listener is**

1. On the node's own port, beside the client protocol: one more route on
   the server that holds every device's session; a flood of hook
   requests competes with the page.
2. A separate listener, off unless asked, loopback by default: a person
   who wants the internet to reach it puts their own proxy or tunnel in
   front (as `--bind` already asks), and the node's own port is
   untouched.

**What the agent is given**

1. The body as the prompt (GOAT's default): the sender, or whoever wrote
   the text the sender relays (an issue's comment, a form's field),
   writes the agent's instructions.
2. The hook's own prompt, written by the person who made the hook, then
   the body fenced as data: the instruction is the person's; the body is
   what it works on.

## Decision

- **Standard Webhooks**, option 3. `POST /hooks/<id>`; the signature is
  checked in constant time against the hook's secret (the decoded bytes
  of `whsec_<base64>`, 32 random bytes made by the node); one or more
  `v1,` signatures may be sent, any matching passes; the timestamp must
  be within five minutes of the node's clock; the id is remembered 24
  hours (at most the last 1,024 per hook) and a repeat answers `200`
  with nothing run. Other providers' schemes are not built: a relay (or
  the provider's own Standard Webhooks support) turns one into this.
- **A separate listener**, option 2: `vesseld --hooks [ADDR]`, on
  `127.0.0.1:7080` unless an address is given (7077 is the node's, 7078
  its relay's, 7079 `vessel ui`'s; another address logs a warning). Hooks can be made, listed and rotated without it;
  `hook.list` says `off`, `listening { address }` or `failed`.
- **Checked cheapest first**, so a request that will be refused costs a
  map lookup and at most one HMAC: the method (`405`); the path and the
  hook, in memory (unknown and disabled answer the same `404`, and
  unknown ids share one counter, never one per id); the headers;
  `Content-Length`, then a read capped at 64 KiB (`413`), with 10 s to
  send the headers and 10 s for the body, and at most 32 connections at
  once; the time (`401`); the signature (`401`); the id seen (`200`);
  then the budgets: 30 accepted a minute per hook (`429`), and the
  target's room (a `new_session` hook has at most 4 live sessions, a
  `session` hook's target its queue's bound: `429` with `retry-after`,
  the id not remembered). Then `202`, and the run is made after the
  answer, so a sender's timeout is never the agent's start. A run the
  service refuses forgets its id, so the sender's retry can run.
  Rejected requests are logged at most 10 a minute per hook and never
  take a budget a signed request needs.
- **The secret** is made by the node and returned by `hook.create` and
  `hook.rotate` only (`HookCreated`); it is a file under the node's data
  folder (`hooks/<id>.key`, owner-only), never a database row, never
  logged (`HookSecret`'s `Debug` is redacted); rotating refuses the old
  one at once. The delivery log keeps the time, the id, the size and the
  outcome, never a header or the body. Writing hooks is `admin`, and
  refused to agents and to calls delivered from another node.
- **What the agent is given**, option 2: the hook's prompt, then a
  `vessel:` line saying the request came from outside this machine and
  is data, not instructions, then the body (UTF-8, cut at 16 KiB, else
  only its size) between `<vessel-hook-data-R>` tags whose `R` is random
  per delivery, so the body cannot close the fence. The session's agent
  gets the `read` scope only (it can start no session and make no
  schedule through Vessel) and is offered no questions.

## Consequences, and what a hook cannot make safe

- **The fence is advice to the model, not a control.** A body is text
  that whoever the sender relays may have written: an issue comment from
  anyone on the internet, a form field. A model can follow instructions
  in it despite the fence. What the agent can then do is what its
  harness lets it do: Vessel limits only its own tools (`read`), not the
  agent's shell or file edits, which the session's permission mode
  governs. A mode that asks before every tool keeps a person in the
  loop (the run says `waiting` until someone answers); a mode that does
  not ask lets text from outside run commands on this machine with the
  definer's access. Vessel cannot make that safe; it can say so where
  the mode is chosen, and the page track must (below).
- So, as designed, the node keeps the parts it can make safe (who may
  send, once, recently, how much, how often) and leaves the rest to the
  definer's choice, stated: the same risk as pasting an issue into an
  agent, at a rate a machine sets. Track D judged a stated risk not
  enough to open a door into the machine while the node cannot require
  the mode that asks ("Where it stands").
- A sender's clock more than five minutes off is refused; a sender that
  never signs (a plain token) cannot use a hook without a relay.
- Hooks are bounded per node (64), deliveries kept per hook (100).

## For the page track

- `hook.create` and `hook.rotate` answer the secret once: show it once,
  with a copy control, and never keep it in the page's state after the
  dialog closes.
- Where a hook's session spec chooses a permission mode, say beside it
  what the first consequence above says, in a sentence.
- `hook.list`'s `listener` says whether anything can reach a hook at
  all.

## Where it stands

Track D (2026-11-12) did not build the listener. In place: the wire's
types (`vessel_proto::automation::hook`: `HookSecret` with its redacted
`Debug`, the definitions, deliveries and outcomes), and schema 13's
`hooks`, `hook_deliveries` and `hook_keys` tables with the store's
writes for them. Nothing in the node reads them, no `hook.*` method is in
the RPC table, and no option opens a port, so no client can make a hook
and nothing listens.

What remains, in order:

1. The six methods (`hook.list`, `.create`, `.update`, `.rotate`,
   `.delete`, `.deliveries`; `admin` to write, refused to agents and
   delegated callers), the secret file (`hooks/<id>.key`, owner-only,
   written before the row, removed after it), `vessel hook`.
2. The listener: `--hooks [ADDR]` on `vesseld` and `vessel node start`,
   its own task and connection bound, the checks in the order above,
   the id map and the per-hook budgets in memory, the delivery log.
3. The run: the schedule's runner made as the hook's definer with a
   session whose agent gets `read` only, the fenced body, `202` before
   it is made, the id forgotten on a refused run.
4. Tests on a real node: a signed request runs once; a replay, a stale
   time, a wrong signature, a body over 64 KiB, an unknown and a
   disabled hook are refused with nothing run and nothing logged beyond
   the bound; a full target answers `429`; the secret never appears in a
   log line, a list or an error.

Why not in this track, two reasons:

- The one more time would not fix: the node cannot tell whether a hook
  session's agent asks before it acts. Permission modes are each
  harness's own option values, and a profile's settings can approve
  tools in advance where the node never looks, so Vessel can neither
  pin a hook session to a mode that asks nor refuse a hook whose mode
  does not. Opening the listener now would make the first consequence
  above a door that whatever text a sender relays can push through.
  What would close it: the node knowing, per harness, which option value
  asks before every tool (and a profile saying it approves nothing in
  advance), so `hook.create` can require it.
- The time went to schedules, saved prompts and the starter told, each
  through the whole gate; a door into the machine is the part least
  worth shipping without every test in item 4.
