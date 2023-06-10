//! [`serde_json`] integration.

use std::marker::PhantomData;

/// The decoder for [`serde_json`].
#[derive(Debug)]
pub struct Decoder<T> {
    /// The marker for data type to decode.
    pub data_type: PhantomData<T>,
}

impl<T> Default for Decoder<T> {
    fn default() -> Self {
        Self {
            data_type: PhantomData,
        }
    }
}

impl<T> crate::Decoder for Decoder<T>
where
    T: for<'de> serde::de::Deserialize<'de>,
{
    type Value = T;
    type Error = serde_json::Error;

    fn decode(
        &self,
        buf: &[u8],
    ) -> Result<crate::Decoded<Self::Value>, crate::DecodeError<Self::Error>> {
        let mut iter = serde_json::Deserializer::from_slice(buf).into_iter::<T>();
        let item = iter.next();
        match item {
            None => Err(crate::DecodeError::NeedMoreData),
            Some(Ok(value)) => Ok(crate::Decoded {
                value,
                consumed_bytes: iter.byte_offset(),
            }),
            Some(Err(err)) if err.is_eof() => Err(crate::DecodeError::NeedMoreData),
            Some(Err(err)) => Err(crate::DecodeError::Other(err)),
        }
    }
}
