# Owned access to the existing session metadata mutex

[Decision Log]
- Purpose: retain the SAME metadata serialization across a tracked creation
  capsule after its original request stops waiting.
- Existing constraint: a borrowed guard cannot transfer to an independently owned
  command. Adding another mutex would not order existing transition callers.
- Alternatives: a second lease/registry, a borrowed request guard, or Arc ownership
  of the existing mutex while preserving all existing lock call sites.
- Choice and reason: change only the field to Arc<Mutex<()>> and its constructor
  to Arc::new(Mutex::new(())). Existing .lock().await callers retain the same
  mutex and order. The concrete Sessions factory obtains its own lock_owned.
- Impact: this mechanical change creates no getter, permission, independent lock,
  actor command or positive effect fence. Concrete retained-owner, caller-loss,
  timeout and lock-order checks remain separate. No actor may reacquire a lease
  already retained by its command. This is staged source, not a qualified handoff.
