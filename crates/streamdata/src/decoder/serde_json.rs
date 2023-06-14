//! [`serde_json`] integration.

use std::marker::PhantomData;

/// The decoder for [`serde_json`].
#[derive(Debug)]
pub struct Decoder<T> {
    /// The marker for data type to decode.
    pub data_type: PhantomData<T>,
}

impl<T> Decoder<T> {
    /// Create a new [`Decoder`].
    pub const fn new() -> Self {
        Self {
            data_type: PhantomData,
        }
    }
}

impl<T> Default for Decoder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Buffer> crate::Decoder<Buffer> for Decoder<T>
where
    T: for<'de> serde::de::Deserialize<'de>,
    Buffer: crate::Buffer,
{
    type Value = T;
    type Error = serde_json::Error;

    fn decode(
        &mut self,
        input: &mut Buffer,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq, serde::Deserialize)]
    struct TestObject {
        pub field: String,
    }

    type TO = TestObject;

    // A helper function to make a test object.
    fn make_to(uid: &str) -> TO {
        TO { field: uid.into() }
    }

    fn assert_test_object(
        tested_test_object: Option<Result<TO, serde_json::Error>>,
        expected_uid: &str,
    ) {
        let actual_to = tested_test_object
            .expect("expected an yielded entry, but none found")
            .expect("parsing failed");
        let expected_to = make_to(expected_uid);
        assert_eq!(actual_to, expected_to);
    }

    fn make_state<T>() -> crate::State<Decoder<T>, Vec<u8>> {
        crate::State {
            buffer: Vec::new(),
            decoder: Decoder::default(),
        }
    }

    #[test]
    fn test_empty() {
        let dec = make_state::<TO>();
        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_incomplete() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(b"{");
            assert!(stream.next().is_none());
        }

        assert_eq!(dec.finish().unwrap_err(), b"{");
    }

    #[test]
    fn test_rubblish() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(b"qwerty");
            assert!(stream.next().unwrap().is_err());
            assert!(stream.next().is_none());
        }

        assert_eq!(dec.finish().unwrap_err(), b"qwerty");
    }

    #[test]
    fn test_one() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(
                br#"{
                    "field": "val0"
                }"#,
            );
            assert_test_object(stream.next(), "val0");
            assert!(stream.next().is_none());
        }

        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_chunked() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(
                br#"{
                    "fie"#,
            );
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"ld": "val0"
                }"#,
            );
            assert_test_object(stream.next(), "val0");
            assert!(stream.next().is_none());
        }

        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_two() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(
                br#"{
                    "field": "val0"
                }{
                    "field": "val1"
                }"#,
            );
            assert_test_object(stream.next(), "val0");
            assert_test_object(stream.next(), "val1");
            assert!(stream.next().is_none());
        }

        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_many_chunked_1() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(
                br#"{
                    "fie"#,
            );
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"ld": "val0"
                }{
                    "fi"#,
            );
            assert_test_object(stream.next(), "val0");
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"eld":"val1"
                }"#,
            );
            assert_test_object(stream.next(), "val1");
            assert!(stream.next().is_none());
        }

        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_many_chunked_2() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(
                br#"{
                    "field": "val0"
                }{
                    "fie"#,
            );
            assert_test_object(stream.next(), "val0");
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"ld": "val1"
                }{
                    "field": "val2"
                }{
                    "fi"#,
            );
            assert_test_object(stream.next(), "val1");
            assert_test_object(stream.next(), "val2");
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"eld": "val3"
                }{
                    "field": "val4"
                }"#,
            );
            assert_test_object(stream.next(), "val3");
            assert_test_object(stream.next(), "val4");
            assert!(stream.next().is_none());
        }

        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_two_one_by_one() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(
                br#"{
                    "field": "val0"
                }"#,
            );
            assert_test_object(stream.next(), "val0");
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"{
                    "field": "val1"
                }"#,
            );
            assert_test_object(stream.next(), "val1");
            assert!(stream.next().is_none());
        }

        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_incomplete_after_valid_data() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(
                br#"{
                    "field": "val0"
                }{"#,
            );
            assert_test_object(stream.next(), "val0");
            assert!(stream.next().is_none());
        }

        assert_eq!(dec.finish().unwrap_err(), b"{");
    }

    #[test]
    fn test_does_not_allow_unparsed_newlines_at_finish() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(b"\n");
            assert!(stream.next().is_none());
        }

        assert_eq!(dec.finish().unwrap_err(), b"\n");
    }

    #[test]
    fn test_does_not_allow_two_unparsed_newlines_at_finish() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(b"\n\n");
            assert!(stream.next().is_none());
        }

        assert_eq!(dec.finish().unwrap_err(), b"\n\n");
    }

    #[test]
    fn test_does_not_allow_three_unparsed_newlines_at_finish() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(b"\n\n\n");
            assert!(stream.next().is_none());
        }

        assert_eq!(dec.finish().unwrap_err(), b"\n\n\n");
    }

    #[test]
    fn test_memory_usage() {
        let mut dec = make_state::<TO>();

        let chunk = br#"{
            "field": "val0"
        }"#;
        let mut chunks = chunk.iter().cycle();

        let max_chunks_per_iter = 15;

        // Simulate processing a huge number of items.
        for _ in 0..100_000 {
            // Take random amout of bytes from the chunks iter and prepare the
            // next chunk.
            let to_take = rand::random::<usize>() % (chunk.len() * max_chunks_per_iter);
            let next_chunk = (&mut chunks).take(to_take).copied().collect::<Box<_>>();

            // Process the chunk data.
            let stream = dec.process_next_chunk(next_chunk.as_ref());
            stream.try_drain().unwrap(); // consume all the emitted items
        }

        // Check that `buffer` capacity didn't grow out way of hand.
        // If we had issues with memory management, it would be the one
        // to blow first.
        let actual_capacity = dec.buffer.capacity();
        let estimated_capacity_upper_bound = chunk.len() * 100;
        assert!(actual_capacity <= estimated_capacity_upper_bound,
            "actual capacity {actual_capacity} is over the estimated upper bound of {estimated_capacity_upper_bound}");
    }
}
