//! A method's shape: the method table's last column.

/// How a method is answered, and whether it carries another call.
///
/// [Decision Log]
/// - Purpose: three places need this per method: the client may make only
///   a one-reply call with `Connection::call`, the node's guard hands the
///   envelope methods itself so they can run the call they carry, and the
///   node's record of failed calls (ADR 0074) must count a carried call
///   once, not again as its envelope.
/// - Alternative: each place keeps its own list of the exceptions (the
///   client and the guard did). A new method then has to be added to each
///   list by hand, and a reader of one list cannot see the others.
/// - Choice: one column of the method table, so a new method states its
///   shape where it is defined and nothing else repeats it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodShape {
    /// One reply.
    Unary,
    /// The reply opens a stream of frames (`session.subscribe`,
    /// `terminal.subscribe`).
    Stream,
    /// Carries another call to a node and answers once with that call's
    /// reply (`fleet.forward`, `fleet.relay`, `fleet.deliver`). Whether it
    /// failed is the carried call's outcome.
    Envelope,
}
