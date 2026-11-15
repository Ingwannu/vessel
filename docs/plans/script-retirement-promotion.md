# Conditional shared retirement promotion

This portable delta promotes the reviewed layout owner hook from `97d29b55` on
announced `82ca07a7`. It carries no Scripts renderer, new process manager, root
protocol change or marker registration. It is conditional on a real feature
owner registering its callback in the same consumer union. Empty `Owners::new`
and duplicate-owner refusal remain unchanged. Without a real caller this is
not a standalone compiled producer or product-ready feature.

[Decision Log]
- Purpose: separate the common owner-hook promotion from the Scripts feature,
  so other real owners can adopt one shared mechanism without its UI branch.
- Existing assets: the sole layout book and synchronous post-commit callbacks.
- Alternatives: per-feature target counters, event queues or generic auto hooks.
- Choice: enable only the module, field and existing local-mutation call sites;
  retain test-only canonical-preferences/resident parents. Canonical source and
  reason variants stay gated with their absent product caller.
- Why: a callback must retire actual owner admissions before another callback,
  while preserving original detached writes and uncertain outcomes. Metadata
  retirement grants no stop, retry, discard or replacement-resource authority.
- Effects: every real consumer owes its scoped source/runtime union and the
  lead gate. Current-target equality alone remains no ABA proof.
