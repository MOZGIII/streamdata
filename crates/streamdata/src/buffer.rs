//! Buffer implementations.

/// [`Buffer`] captures the interface we require from the piece that maintains
/// the [`crate::State`] buffer.
/// This buffer is intended for keeping the undecoded partial chunks.
pub trait Buffer {
    /// Append the data to the end of the buffer.
    fn append(&mut self, chunk: &[u8]);
    /// View the current contents of the buffer.
    fn view(&self) -> &[u8];
    /// Drop the given amout of bytes from the start of the buffer.
    fn advance(&mut self, bytes: usize);
}

impl super::Buffer for Vec<u8> {
    fn append(&mut self, chunk: &[u8]) {
        self.extend_from_slice(chunk)
    }

    fn view(&self) -> &[u8] {
        self
    }

    fn advance(&mut self, bytes: usize) {
        self.drain(..bytes);
    }
}

#[cfg(feature = "bytes")]
impl super::Buffer for bytes::BytesMut {
    fn append(&mut self, chunk: &[u8]) {
        bytes::BufMut::put_slice(self, chunk)
    }

    fn view(&self) -> &[u8] {
        bytes::Buf::chunk(self)
    }

    fn advance(&mut self, bytes: usize) {
        bytes::Buf::advance(self, bytes)
    }
}
