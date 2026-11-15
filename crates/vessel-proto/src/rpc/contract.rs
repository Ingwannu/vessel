//! Request/reply association is independent of method registration.

use super::{Call, Method, Reply};
use serde::{Serialize, de::DeserializeOwned};

/// A request type, bound to its response type and its method.
pub trait Request: Serialize + DeserializeOwned + Send + 'static {
    /// The method this request invokes.
    const METHOD: Method;
    /// What a successful call returns.
    type Response: Serialize + DeserializeOwned + Send + 'static;

    /// Wrap into the generic call enum for sending.
    fn into_call(self) -> Call;

    /// Extract this request's response from a generic reply.
    ///
    /// # Errors
    /// Fails if the reply belongs to a different method, which means the
    /// peer is broken.
    fn response_from(reply: Reply) -> Result<Self::Response, UnexpectedReply>;
}

/// A reply arrived for a different method than the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnexpectedReply {
    /// Method that was called.
    pub expected: Method,
    /// Method the reply belongs to.
    pub got: Method,
}

impl std::fmt::Display for UnexpectedReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "reply for `{}` received for a `{}` request",
            self.got.as_str(),
            self.expected.as_str()
        )
    }
}

impl std::error::Error for UnexpectedReply {}
