//! Various reader implementations.

#[cfg(any(feature = "tokio", feature = "futures"))]
pub mod generic;
#[cfg(feature = "http-body")]
pub mod http_body;
