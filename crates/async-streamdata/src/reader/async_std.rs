//! [`async_std`] integration.

use async_std::io::ReadExt;

/// A reader with an internal buffer.
#[derive(Debug)]
pub struct BoxBufReader<T> {
    /// The inner reader.
    pub inner: T,
    /// The buffer to use while reading the data.
    pub buf: Box<[u8]>,
}

#[cfg_attr(feature = "async-trait", async_trait::async_trait)]
impl<T> crate::Reader for BoxBufReader<T>
where
    T: async_std::io::Read + Unpin,
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
