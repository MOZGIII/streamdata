//! Various reader implementations.

#[cfg(any(feature = "tokio", feature = "futures-io"))]
pub mod generic;
#[cfg(feature = "http-body")]
pub mod http_body;
#[cfg(feature = "quinn")]
pub mod quinn;
#[cfg(feature = "futures-core")]
pub mod stream;
