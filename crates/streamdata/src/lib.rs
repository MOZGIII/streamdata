//! Utilities for decoding stream data.

mod buffer;
pub mod decoder;

pub use buffer::*;

/// The managed decoding state for the stream of data.
#[derive(Debug)]
pub struct State<Decoder, Buffer> {
    /// The decoder to use for processing the data.
    pub decoder: Decoder,
    /// The buffer containing carried data from the previously decoded chunks.
    pub buffer: Buffer,
}

/// The decoder error.
/// This can either be any error that indicates that more data is needed
/// to decode the value, i.e. the data is empty or is an incomplete part of
/// an encoded value.
pub enum DecodeError<T> {
    /// More data is required for successful decoding.
    NeedMoreData,
    /// The decoder was unable to decode the data, but it has deremined
    /// the correct action to proceed would be to drop some bytes from
    /// the buffer and try again.
    SkipData(usize),
    /// Some other error has occured.
    Other(T),
}

/// The results of the successful decoding.
pub struct Decoded<T> {
    /// The decoded value.
    pub value: T,
    /// The amount of bytes consumed from the buffer.
    pub consumed_bytes: usize,
}

/// [`Decoder`] represents the ability to decode a value from a given buffer
/// of data.
pub trait Decoder<Input>
where
    Input: self::Buffer,
{
    /// The value to decode.
    type Value;

    /// The error that can occur while decoding the value.
    type Error;

    /// Decode (up to one) value from the buffer, returning the decoded value
    /// accompanied by the amount of bytes consumed from the `buf` on success,
    /// or a relevant decoding error.
    fn decode(&mut self, input: &mut Input) -> Result<Self::Value, DecodeError<Self::Error>>;
}

impl<Decoder, Buffer> State<Decoder, Buffer>
where
    Decoder: self::Decoder<Buffer>,
    Buffer: self::Buffer,
{
    /// Take the next chunk of data and return the iterator over the values
    /// available with this new data.
    pub fn process_next_chunk(&mut self, chunk: &[u8]) -> AvailableIter<'_, Decoder, Buffer> {
        self.buffer.append(chunk);
        AvailableIter::new(self)
    }

    /// Returns `true` if there is no bufferred data.
    ///
    /// When the [`State`] buffer is not empty, this means that it contains
    /// some (possibly incomplete) data for the encoded values that has been
    /// added to this state previously but has not yet been decoded (the most
    /// obvious reason for which is that decoding would require more data
    /// to arrive to succeed).
    pub fn is_empty(&self) -> bool {
        self.buffer.view().is_empty()
    }

    /// Finish the processing.
    ///
    /// Returns `Ok(())` if the state is empty [see [`Self::is_empty`],
    /// otherwise returns an `Err` with the buffer containing the unhandled
    /// data.
    pub fn finish(self) -> Result<(), Buffer> {
        if self.is_empty() {
            return Ok(());
        }
        Err(self.buffer)
    }
}

/// Iterate over the data readily available in the state, decoding the values
/// on the fly.
///
/// When the error is returned, the further progress is no longer possible
/// because the iterator will (most certainly, but actually depending on
/// the `Decoder` implementation) be stuck, trying to decode the same data over
/// and over again. The way to continue from this situation is to give up on
/// the iterator, and either give up on the whole [`State`]
/// (by [`Self::finish`]-ing it) or try to correct the state by altering
/// the `buffer` somehow.
///
/// This can be ergonomic when used with `.collect::<Result<Vec<_>, _>`.
pub struct AvailableIter<'state, Decoder, Buffer>
where
    Decoder: self::Decoder<Buffer>,
    Buffer: self::Buffer,
{
    /// A reference to the state.
    /// The fact that we are holding this reference prevents anything from
    /// happening to the state until we are dropped.
    state: &'state mut State<Decoder, Buffer>,
    /// Short circut on error.
    short_circut: bool,
}

impl<'state, Decoder, Buffer> Iterator for AvailableIter<'state, Decoder, Buffer>
where
    Decoder: self::Decoder<Buffer>,
    Buffer: self::Buffer,
{
    type Item = Result<
        <Decoder as self::Decoder<Buffer>>::Value,
        <Decoder as self::Decoder<Buffer>>::Error,
    >;

    fn next(&mut self) -> Option<Self::Item> {
        if self.short_circut {
            return None;
        }
        loop {
            return match self.state.decoder.decode(&mut self.state.buffer) {
                Ok(value) => Some(Ok(value)),
                Err(DecodeError::NeedMoreData) => None,
                Err(DecodeError::SkipData(bytes_to_skip)) => {
                    self.state.buffer.advance(bytes_to_skip);
                    continue; // skip return
                }
                Err(DecodeError::Other(error)) => {
                    self.short_circut = true;
                    Some(Err(error))
                }
            };
        }
    }
}

impl<'state, Decoder, Buffer> AvailableIter<'state, Decoder, Buffer>
where
    Decoder: self::Decoder<Buffer>,
    Buffer: self::Buffer,
{
    /// Create a new [`Self`] for a given state.
    /// Private fn for internal use only.
    fn new(state: &'state mut State<Decoder, Buffer>) -> Self {
        Self {
            state,
            short_circut: false,
        }
    }

    /// View access to the state buffer.
    ///
    /// This can be useful for inspecting the buffer after the chunks have beed added to it.
    pub fn buffer(&self) -> &Buffer {
        &self.state.buffer
    }

    /// Decode and drop all available data, or fail with the first encountered
    /// decoding error.
    pub fn try_drain(self) -> Result<(), <Decoder as self::Decoder<Buffer>>::Error> {
        for result in self {
            let _ = result?;
        }
        Ok(())
    }

    /// Decode and collect all available data, or fail with the first
    /// encountered decoding error.
    pub fn try_collect<T>(self) -> Result<T, <Decoder as self::Decoder<Buffer>>::Error>
    where
        T: FromIterator<<Decoder as self::Decoder<Buffer>>::Value>,
    {
        self.collect()
    }
}
