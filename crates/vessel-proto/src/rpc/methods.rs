//! The method table's macro: one row per method generates `Method`, the
//! `Call` and `Reply` enums, the `Request` impls, the `Handler` trait and
//! `dispatch` (the parent module's Decision Log says why). The table
//! itself is the parent's `rpc_methods!` invocation, the one place the
//! protocol is defined; the names the expansion uses resolve there.

// Authentication is a requirement, not an additional grantable scope.
// Keeping it in the table avoids a hidden exception in the node's guard.
macro_rules! method_scope {
    (Authenticated) => {
        None
    };
    ($scope:ident) => {
        Some(Scope::$scope)
    };
}

// A row's protocol column: a number, `{expression}` for a named version
// constant, `(pending FEATURE)` for a method whose number is not assigned
// yet, or `(staged FEATURE)` for a pending method that has no runtime yet
// (below). `FEATURE` is the feature's `Option<ProtocolVersion>` constant
// (A6): a pending method has a row like any other, and both ends refuse it
// while the constant is None, so no track invents a version to make its
// row compile.
macro_rules! method_protocol {
    ((pending $feature:path)) => {
        $feature
    };
    ((staged $feature:path)) => {
        $feature
    };
    ({ $since:expr }) => {
        Some(ProtocolVersion::new($since))
    };
    ($since:literal) => {
        Some(ProtocolVersion::new($since))
    };
}

// Whether a row is staged: its values (request, response, row) landed
// before the package that runs it.
macro_rules! method_staged {
    ((staged $feature:path)) => {
        true
    };
    ($protocol:tt) => {
        false
    };
}

// One `Handler` method per row.
//
// [Decision Log: staged rows] (2026-11-15, G1's values-only leaf)
// - Purpose: a values-only package (its request and response types and
//   its table row, no runtime by rule) must be able to land before the
//   package that serves the method.
// - Before: every row, pending ones too, was a required `Handler` method,
//   and the node's guard forwards each one to a `NodeService` method of
//   the same name, so a row without its runtime broke the node's build.
// - Alternatives: a default body for every pending row (the guard would
//   then have to skip pending rows, and a pending method that has its
//   runtime would answer Unsupported once its number is assigned); a stub
//   service method in the node per leaf (runtime code in a values package,
//   which the staging rule forbids); a dummy version (refused, A6).
// - Choice: a third protocol form, `(staged FEATURE)`. Its `Handler` method
//   has a default body answering the typed Unsupported, the node's guard
//   generates nothing for it, and every other row stays a required method,
//   so an assigned or pending row still does not compile without its
//   handler. The package that wires the method changes `staged` to
//   `pending` in its row in the same commit as the handler.
// - Cost: one more row form to read. A staged row may not be assigned a
//   number (it would be callable while nothing serves it): its default
//   body asserts at compile time that its constant is None, and
//   `Method::is_staged` lets a test check the table.
macro_rules! handler_method {
    ((staged $feature:path); $(#[$doc:meta])*; $handler:ident; $request:ty; $response:ty) => {
        $(#[$doc])*
        ///
        /// Staged: no node serves this method yet, so by default it answers
        /// `Unsupported`; the package that wires it overrides this.
        fn $handler(
            &self,
            _context: &Self::Context,
            _request: $request,
        ) -> impl Future<Output = Result<$response, WireError>> + Send {
            // Nothing serves a staged method, so it may not be callable:
            // its feature constant stays None until the row is wired.
            const _: () = assert!(
                Option::is_none(&$feature),
                "a staged row is wired (`pending`) before its number is assigned"
            );
            // Only the method is named: the request's values stay out of
            // the answer and the log.
            std::future::ready(Err(WireError::new(
                crate::error::ErrorCode::Unsupported,
                format!(
                    "`{}` is not served by this node yet",
                    <$request as Request>::METHOD.as_str()
                ),
            )))
        }
    };
    ($protocol:tt; $(#[$doc:meta])*; $handler:ident; $request:ty; $response:ty) => {
        $(#[$doc])*
        fn $handler(
            &self,
            context: &Self::Context,
            request: $request,
        ) -> impl Future<Output = Result<$response, WireError>> + Send;
    };
}

// A row's shape column (see `MethodShape`).
macro_rules! method_shape {
    (unary) => {
        MethodShape::Unary
    };
    (stream) => {
        MethodShape::Stream
    };
    (envelope) => {
        MethodShape::Envelope
    };
}

/// Generates this crate's per-method code from the method table's rows
/// (`method_table!` in the parent module, which documents the row).
///
/// The scope column (protocol 4, ADR 0010) is what a paired device must hold
/// to call the method; `Authenticated` explicitly requires no extra scope.
/// Every call still requires an admitted connection. Adding an unclassified
/// method does not compile. The protocol column (protocol 5) is the first
/// protocol version with the method: the client refuses to send it on an
/// older connection and the node refuses to run it there, so neither side
/// relies on the other ignoring what it cannot know.
macro_rules! rpc_methods {
    ($(
        $(#[$doc:meta])*
        $variant:ident / $handler:ident / $wire:literal [$scope:ident, $protocol:tt, $shape:ident] : $request:ty => $response:ty;
    )*) => {
        /// Every RPC method.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        pub enum Method {
            $(
                $(#[$doc])*
                #[serde(rename = $wire)]
                $variant,
            )*
        }

        impl Method {
            /// Wire name.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( Self::$variant => $wire, )*
                }
            }

            /// The additional scope needed on an authenticated connection.
            /// None never means anonymous access (ADR 0044).
            #[must_use]
            pub const fn scope(self) -> Option<Scope> {
                match self {
                    $( Self::$variant => method_scope!($scope), )*
                }
            }

            /// The first protocol version with this method, or None while
            /// its number is pending: such a method is refused on every
            /// connection rather than given a stand-in version.
            #[must_use]
            pub const fn since(self) -> Option<ProtocolVersion> {
                match self {
                    $( Self::$variant => method_protocol!($protocol), )*
                }
            }

            /// Whether this method is staged: its row and values are here,
            /// but no node serves it yet (`(staged FEATURE)`).
            #[must_use]
            pub const fn is_staged(self) -> bool {
                match self {
                    $( Self::$variant => method_staged!($protocol), )*
                }
            }

            /// How this method is answered, and whether it carries another
            /// call.
            #[must_use]
            pub const fn shape(self) -> MethodShape {
                match self {
                    $( Self::$variant => method_shape!($shape), )*
                }
            }

            /// Every method, in table order.
            pub const ALL: &'static [Self] = &[$( Self::$variant, )*];
        }

        /// A request of any method, as sent on the wire.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(tag = "method", content = "params")]
        pub enum Call {
            $(
                $(#[$doc])*
                #[serde(rename = $wire)]
                $variant($request),
            )*
        }

        impl Call {
            /// Which method this is.
            #[must_use]
            pub const fn method(&self) -> Method {
                match self {
                    $( Self::$variant(_) => Method::$variant, )*
                }
            }
        }

        /// A successful response of any method, as sent on the wire.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(tag = "method", content = "result")]
        pub enum Reply {
            $(
                $(#[$doc])*
                #[serde(rename = $wire)]
                $variant($response),
            )*
        }

        impl Reply {
            /// Which method this answers.
            #[must_use]
            pub const fn method(&self) -> Method {
                match self {
                    $( Self::$variant(_) => Method::$variant, )*
                }
            }
        }

        $(
            impl Request for $request {
                const METHOD: Method = Method::$variant;
                type Response = $response;

                fn into_call(self) -> Call {
                    Call::$variant(self)
                }

                fn response_from(reply: Reply) -> Result<$response, UnexpectedReply> {
                    let got = reply.method();
                    if let Reply::$variant(response) = reply {
                        Ok(response)
                    } else {
                        Err(UnexpectedReply { expected: Method::$variant, got })
                    }
                }
            }
        )*

        /// Server side of the protocol: one method per RPC.
        ///
        /// `Context` is per-connection state (for example the connection's
        /// subscriptions), so connection-scoped methods such as `subscribe`
        /// go through the same exhaustive table as everything else.
        ///
        /// Every assigned or pending row is a method its implementor must
        /// write; only a staged row (`(staged FEATURE)`) has a default,
        /// which answers `Unsupported`. So this does not compile:
        ///
        /// ```compile_fail,E0046
        /// struct Nothing;
        ///
        /// impl vessel_proto::Handler for Nothing {
        ///     type Context = ();
        /// }
        /// ```
        pub trait Handler: Sync {
            /// Per-connection state passed to every call.
            type Context: Sync;

            $(
                handler_method!($protocol; $(#[$doc])*; $handler; $request; $response);
            )*
        }

        /// Route a call to its handler method. Exhaustive: a method without
        /// a handler does not compile.
        pub async fn dispatch<H: Handler>(handler: &H, context: &H::Context, call: Call) -> Outcome {
            match call {
                $(
                    Call::$variant(request) => match handler.$handler(context, request).await {
                        Ok(response) => Outcome::Ok(Reply::$variant(response)),
                        Err(error) => Outcome::Err(error),
                    },
                )*
            }
        }
    };
}
