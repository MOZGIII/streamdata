//! [`quinn`] integration.

/// An [`quinn::RecvStream`] reader.
#[derive(Debug)]
pub struct Reader {
    /// The stream to read the data from.
    pub recv_stream: quinn::RecvStream,
    /// The max length of *a single read* the system will attempt to do.
    /// This parameter is akin to the buffer size, but is not quite exactly that.
    pub max_length: usize,
}

#[cfg_attr(feature = "async-trait", async_trait::async_trait)]
impl crate::Reader for Reader {
    type Data<'data> = bytes::Bytes;
    type Error = quinn::ReadError;

    async fn next(&mut self) -> Option<Result<Self::Data<'_>, Self::Error>> {
        let result = self.recv_stream.read_chunk(self.max_length, true).await;
        match result {
            Ok(Some(chunk)) => Some(Ok(chunk.bytes)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}
