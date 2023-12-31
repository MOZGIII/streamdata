//! The async stream data decoding utilities.
//!
//! This crate provides [`streamdata`] adaptors for `futures`, `tokio` and
//! `async-std`.

#![cfg_attr(
    not(feature = "no-async-fn-in-trait-feature"),
    allow(incomplete_features)
)]
#![cfg_attr(
    not(feature = "no-async-fn-in-trait-feature"),
    feature(async_fn_in_trait)
)]

pub mod reader;

use async_stream::try_stream;
use bytes::Buf;
use futures_core::Stream;

/// A type alias for a more compact [`Error`] declaration.
pub type ErrorFor<Reader, Decoder, Buffer> =
    Error<<Reader as self::Reader>::Error, <Decoder as streamdata::Decoder<Buffer>>::Error, Buffer>;

/// A type alias for a more compact [`Result`] declaration.
pub type ResultFor<Reader, Decoder, Buffer> =
    Result<<Decoder as streamdata::Decoder<Buffer>>::Value, ErrorFor<Reader, Decoder, Buffer>>;

/// [`Reader`] provides an abstract interface to the various async read
/// implementations.
///
/// The goal of the reader is to read the data. It will return all the data
/// chunks in a loop.
#[cfg_attr(feature = "async-trait", async_trait::async_trait)]
pub trait Reader {
    /// The chunk data.
    type Data<'data>: bytes::Buf;
    /// The error that can originate at the reader.
    type Error;

    /// Read next chunk of data.
    /// If there is no more data to read - returns `None`.
    async fn next(&mut self) -> Option<Result<Self::Data<'_>, Self::Error>>;
}

/// Converts the given reader to a stream of decoded values.
pub fn stream<Reader, Decoder, Buffer>(
    mut reader: Reader,
    mut state: streamdata::State<Decoder, Buffer>,
) -> impl Stream<Item = ResultFor<Reader, Decoder, Buffer>>
where
    Reader: self::Reader,
    Decoder: streamdata::Decoder<Buffer>,
    Buffer: streamdata::Buffer,
{
    try_stream! {
        while let Some(data) = reader.next().await {
            let data = data.map_err(Error::Reading)?;
            let results = state.process_next_chunk(data.chunk());
            for result in results {
                let value = result.map_err(Error::Decoding)?;
                yield value;
            }
        }
        state.finish().map_err(|data| Error::UndecodedDataLeftUponCompletion { data })?;
    }
}

/// Errors that can occur while decoding the data stream.
#[derive(Debug, thiserror::Error)]
pub enum Error<ReaderError, DecoderError, Buffer> {
    /// An error has occured while reading the data.
    #[error("reading: {0}")]
    Reading(#[source] ReaderError),
    /// An error has occured while decoding the values.
    #[error("decoding: {0}")]
    Decoding(#[source] DecoderError),
    /// After all the data was read and the decoding was complete there is still
    /// some data in the state buffer.
    #[error("some data left in the buffer after the data was read completely")]
    UndecodedDataLeftUponCompletion {
        /// The buffer contatining the leftover data.
        data: Buffer,
    },
}
