//! [`tokio_util`] integration (i.e. [`tokio_util::codec::Decoder`] support).

use crate::Buffer;

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

impl<T> crate::Decoder<'static, Vec<u8>> for Decoder<T>
where
    T: tokio_util::codec::Decoder,
{
    type Value = <T as tokio_util::codec::Decoder>::Item;
    type Error = <T as tokio_util::codec::Decoder>::Error;

    #[allow(clippy::arithmetic_side_effects)]
    fn decode<'input>(
        &mut self,
        input: &'input mut Vec<u8>,
    ) -> Result<Self::Value, crate::DecodeError<Self::Error>>
    where
        'static: 'input,
    {
        let buf = input.view();
        let mut buf_bytes = bytes::BytesMut::from(buf);
        match tokio_util::codec::Decoder::decode(&mut self.inner, &mut buf_bytes) {
            Ok(None) => Err(crate::DecodeError::NeedMoreData),
            Ok(Some(value)) => {
                input.advance(buf.len() - buf_bytes.len());
                Ok(value)
            }
            Err(err) => Err(crate::DecodeError::Other(err)),
        }
    }
}
