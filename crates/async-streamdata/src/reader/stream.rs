//! [`futures_core::Stream`] integration.

use futures_util::StreamExt;

/// A [`futures_core::Stream`] reader.
///
/// Allows reading any [`futures_core::Stream`] that
/// returns [`Result<T, E>`]s where `T` implements [`bytes::Buf`].
#[derive(Debug)]
pub struct Reader<T>(pub T);

#[cfg_attr(feature = "async-trait", async_trait::async_trait)]
impl<Stream, Data, Error> crate::Reader for Reader<Stream>
where
    Data: bytes::Buf,
    Stream: futures_core::Stream<Item = Result<Data, Error>> + Unpin,
{
    type Data<'data> = Data;
    type Error = Error;

    async fn next(&mut self) -> Option<Result<Self::Data<'_>, Self::Error>> {
        self.0.next().await
    }
}
