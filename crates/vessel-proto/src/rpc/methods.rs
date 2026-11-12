//! The method table's macro: one row per method generates `Method`, the
//! `Call` and `Reply` enums, the `Request` impls, the `Handler` trait and
//! `dispatch` (the parent module's Decision Log says why). The table
//! itself is the parent's `rpc_methods!` invocation, the one place the
//! protocol is defined; the names the expansion uses resolve there.

/// Declares every RPC method in one table.
///
/// Each row: `Variant / handler_fn / "wire.name" [Scope, since]: Request => Response;`
/// The scope column (protocol 4, ADR 0010) is what a paired device must hold
/// to call the method; adding a method without one does not compile. `since`
/// (protocol 5) is the first protocol version with the method: the client
/// refuses to send it on an older connection and the node refuses to run it
/// there, so neither side relies on the other ignoring what it cannot know.
macro_rules! rpc_methods {
    ($(
        $(#[$doc:meta])*
        $variant:ident / $handler:ident / $wire:literal [$scope:ident, $since:literal] : $request:ty => $response:ty;
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

            /// The scope a paired device needs to call it.
            #[must_use]
            pub const fn scope(self) -> Scope {
                match self {
                    $( Self::$variant => Scope::$scope, )*
                }
            }

            /// The first protocol version with this method.
            #[must_use]
            pub const fn since(self) -> ProtocolVersion {
                match self {
                    $( Self::$variant => ProtocolVersion::new($since), )*
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
        pub trait Handler: Sync {
            /// Per-connection state passed to every call.
            type Context: Sync;

            $(
                $(#[$doc])*
                fn $handler(
                    &self,
                    context: &Self::Context,
                    request: $request,
                ) -> impl Future<Output = Result<$response, WireError>> + Send;
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
