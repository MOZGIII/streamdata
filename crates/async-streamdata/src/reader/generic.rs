//! A generic reader providing `tokio`, `futures` and `async-std` integration.
//!
//! A general reader abstraction for `AsyncRead`-kind of IOs.

use std::marker::PhantomData;

/// A marker trait for the types that specify an IO implmentation.
///
/// You can implement support for a custom IO implementation if needed, to do that provide a struct
/// that implements the [`IoImplementation`].
pub trait IoImplementation {}

/// [`tokio::io::AsyncRead`] integration.
#[cfg(feature = "tokio")]
pub struct Tokio;
#[cfg(feature = "tokio")]
impl IoImplementation for Tokio {}

/// [`futures_io::AsyncRead`] integration.
#[cfg(feature = "futures")]
pub struct Futures;
#[cfg(feature = "futures")]
impl IoImplementation for Futures {}

/// Logic for determining whether an EOF condition was reached.
///
/// This trait may be subject to change if we discover cases where it makes sense to look
/// at `Inner` in particular (its implementation details) to determine whether the EOF has
/// occured. This may be a thing for readers that *know when to stop*.
pub trait EofCondition<Result> {
    /// Returns `true` if the given read was an EOF.
    fn is_eof(read_result: &Result) -> bool;
}

/// An [`EofCondition`] implementation that will treat any `0`-byte reads as EOF.
pub struct ZeroReadEof;

impl<E> EofCondition<Result<usize, E>> for ZeroReadEof {
    fn is_eof(read_result: &Result<usize, E>) -> bool {
        matches!(read_result, Ok(ref n) if *n == 0)
    }
}

/// A reader with an internal buffer.
#[derive(Debug)]
pub struct Reader<IoImplementation, Inner, EofCondition, Buffer> {
    /// The inner reader.
    pub inner: Inner,
    /// The buffer to use for reading the data.
    pub buf: Buffer,
    /// The phantom data for tracking the generic parameters that are not used directly in
    /// the struct definition.
    pub phantom_data: PhantomData<(EofCondition, IoImplementation)>,
}

impl<IoImplementation, Inner, EofCondition, Buffer>
    Reader<IoImplementation, Inner, EofCondition, Buffer>
{
    /// Create an instance of a [`Reader`] with the specified parameters.
    pub fn new(inner: Inner, buf: Buffer) -> Self {
        Self {
            inner,
            buf,
            phantom_data: PhantomData,
        }
    }
}

#[cfg(feature = "tokio")]
#[cfg_attr(feature = "async-trait", async_trait::async_trait)]
impl<Inner, EofCondition> crate::Reader for Reader<Tokio, Inner, EofCondition, bytes::BytesMut>
where
    Inner: tokio::io::AsyncRead + Unpin,
    EofCondition: self::EofCondition<Result<usize, std::io::Error>>,
{
    type Data<'data> = bytes::Bytes;
    type Error = std::io::Error;

    async fn next(&mut self) -> Option<Result<Self::Data<'_>, Self::Error>> {
        use bytes::Buf;
        use tokio::io::AsyncReadExt;

        let result = self.inner.read_buf(&mut self.buf).await;
        if EofCondition::is_eof(&result) {
            return None;
        }
        match result {
            Err(err) => Some(Err(err)),
            Ok(n) => Some(Ok(self.buf.copy_to_bytes(n))),
        }
    }
}

#[cfg(feature = "futures")]
#[cfg_attr(feature = "async-trait", async_trait::async_trait)]
impl<Inner, EofCondition> crate::Reader for Reader<Futures, Inner, EofCondition, Vec<u8>>
where
    Inner: futures_io::AsyncRead + Unpin,
    EofCondition: self::EofCondition<Result<usize, std::io::Error>>,
{
    type Data<'data> = &'data [u8];
    type Error = std::io::Error;

    async fn next(&mut self) -> Option<Result<Self::Data<'_>, Self::Error>> {
        use futures_util::AsyncReadExt;

        let result = self.inner.read(&mut self.buf).await;
        if EofCondition::is_eof(&result) {
            return None;
        }
        match result {
            Err(err) => Some(Err(err)),
            Ok(n) => Some(Ok(&self.buf[..n])),
        }
    }
}
