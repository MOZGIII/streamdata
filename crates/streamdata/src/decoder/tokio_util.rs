//! [`tokio_util`] integration (i.e. [`tokio_util::codec::Decoder`] support).

/// The decoder that wraps any [`tokio_util::codec::Decoder`].
#[derive(Debug)]
pub struct Decoder<T> {
    /// The inner decoder type.
    pub inner: T,
}

impl<T> Decoder<T> {
    /// Create a new [`Decoder`] with a given [`tokio_util::codec::Decoder`].
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> crate::Decoder for Decoder<T>
where
    T: tokio_util::codec::Decoder,
{
    type Value = <T as tokio_util::codec::Decoder>::Item;
    type Error = <T as tokio_util::codec::Decoder>::Error;

    #[allow(clippy::arithmetic_side_effects)]
    fn decode(
        &mut self,
        buf: &[u8],
    ) -> Result<crate::Decoded<Self::Value>, crate::DecodeError<Self::Error>> {
        let mut buf_bytes = bytes::BytesMut::from(buf);
        match tokio_util::codec::Decoder::decode(&mut self.inner, &mut buf_bytes) {
            Ok(None) => Err(crate::DecodeError::NeedMoreData),
            Ok(Some(value)) => Ok(crate::Decoded {
                value,
                consumed_bytes: buf.len() - buf_bytes.len(),
            }),
            Err(err) => Err(crate::DecodeError::Other(err)),
        }
    }
}
