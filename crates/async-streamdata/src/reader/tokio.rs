//! [`tokio`] integration.

use tokio::io::AsyncReadExt;

/// A reader with an internal buffer.
#[derive(Debug)]
pub struct BoxBufReader<T> {
    /// The inner reader.
    pub inner: T,
    /// The buffer to use for reading the data.
    pub buf: Box<[u8]>,
}

impl<T> crate::Reader for BoxBufReader<T>
where
    T: tokio::io::AsyncRead + Unpin,
{
    type Data<'a> = &'a [u8];
    type Error = std::io::Error;

    async fn next(&mut self) -> Option<Result<Self::Data<'_>, Self::Error>> {
        match self.inner.read(self.buf.as_mut()).await {
            Err(err) => Some(Err(err)),
            Ok(n) if n == 0 => None,
            Ok(n) => Some(Ok(&self.buf[..n])),
        }
    }
}

/// A reader with an internal buffer of [`bytes::BytesMut`].
#[derive(Debug)]
pub struct BytesReader<T> {
    /// The inner reader.
    pub inner: T,
    /// The buffer to use for reading the data.
    pub buf: bytes::BytesMut,
}

impl<T> crate::Reader for BytesReader<T>
where
    T: tokio::io::AsyncRead + Unpin,
{
    type Data<'data> = &'data [u8];
    type Error = std::io::Error;

    async fn next(&mut self) -> Option<Result<Self::Data<'_>, Self::Error>> {
        match self.inner.read_buf(&mut self.buf).await {
            Err(err) => Some(Err(err)),
            Ok(n) if n == 0 => None,
            Ok(n) => Some(Ok(&self.buf[..n])),
        }
    }
}
