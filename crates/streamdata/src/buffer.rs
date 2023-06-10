//! Buffer implementations.

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
