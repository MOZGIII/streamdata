//! [`serde_json`] integration.

use std::marker::PhantomData;

use crate::Buffer;

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

impl<T> crate::Decoder<Vec<u8>> for Decoder<T>
where
    T: for<'de> serde::de::Deserialize<'de>,
{
    type Value = T;
    type Error = serde_json::Error;

    fn decode(
        &mut self,
        input: &mut Vec<u8>,
    ) -> Result<Self::Value, crate::DecodeError<Self::Error>> {
        let buf = input.view();
        let mut iter = serde_json::Deserializer::from_slice(buf).into_iter::<T>();
        let item = iter.next();
        match item {
            None => Err(crate::DecodeError::NeedMoreData),
            Some(Ok(value)) => {
                input.advance(iter.byte_offset());
                Ok(value)
            }
            Some(Err(err)) if err.is_eof() => Err(crate::DecodeError::NeedMoreData),
            Some(Err(err)) => Err(crate::DecodeError::Other(err)),
        }
    }
}
