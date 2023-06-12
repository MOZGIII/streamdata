//! [`k8s_openapi`] integration.

use std::marker::PhantomData;

use crate::Buffer;

/// The decoder for [`k8s_openapi::Response`].
#[derive(Debug)]
pub struct Decoder<T> {
    /// The marker for data type to decode.
    pub data_type: PhantomData<T>,
    /// The status code to present to [`k8s_openapi::Response::try_from_parts`].
    pub status_code: k8s_openapi::http::StatusCode,
}

impl<T> Decoder<T> {
    /// Create a [`Decoder`] with the specified status code.
    pub const fn with_status_code(status_code: k8s_openapi::http::StatusCode) -> Self {
        Self {
            data_type: PhantomData,
            status_code,
        }
    }

    /// Create a [`Decoder`] with the predefined status code
    /// [`k8s_openapi::http::StatusCode::OK`].
    pub const fn new() -> Self {
        Self::with_status_code(k8s_openapi::http::StatusCode::OK)
    }
}

impl<T> crate::Decoder<Vec<u8>> for Decoder<T>
where
    T: k8s_openapi::Response,
{
    type Value = T;
    type Error = Error;

    #[allow(clippy::arithmetic_side_effects)]
    fn decode(
        &mut self,
        input: &mut Vec<u8>,
    ) -> Result<Self::Value, crate::DecodeError<Self::Error>> {
        let buf = input.view();
        // Allow skipping over newlines.
        if matches!(buf.first(), Some(b'\n')) {
            return Err(crate::DecodeError::SkipData(1));
        }
        match T::try_from_parts(self.status_code, buf) {
            Ok((value, mut consumed_bytes)) => {
                // Consume one extra character if the newline follows
                // immediately after the response ended.
                if matches!(buf.get(consumed_bytes), Some(b'\n')) {
                    consumed_bytes += 1;
                }
                input.advance(consumed_bytes);
                Ok(value)
            }
            Err(k8s_openapi::ResponseError::NeedMoreData) => Err(crate::DecodeError::NeedMoreData),
            Err(k8s_openapi::ResponseError::Json(err)) => {
                Err(crate::DecodeError::Other(Error::Json(err)))
            }
            Err(k8s_openapi::ResponseError::Utf8(err)) => {
                Err(crate::DecodeError::Other(Error::Utf8(err)))
            }
        }
    }
}

/// The error that can occur when decoding the [`k8s_openapi::Response`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error while deserializing the HTTP response as a JSON value,
    /// indicating the response is malformed.
    #[error("json: {0}")]
    Json(#[source] k8s_openapi::serde_json::Error),

    /// An error while deserializing the HTTP response as a string, indicating that the response data is not UTF-8.
    #[error("utf8: {0}")]
    Utf8(#[source] std::str::Utf8Error),
}

impl From<Error> for k8s_openapi::ResponseError {
    fn from(value: Error) -> Self {
        match value {
            Error::Json(val) => Self::Json(val),
            Error::Utf8(val) => Self::Utf8(val),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use k8s_openapi::{
        api::core::v1::Pod,
        apimachinery::pkg::apis::meta::v1::{ObjectMeta, WatchEvent},
        WatchResponse,
    };

    /// Test object.
    type TO = WatchResponse<Pod>;

    // A helper function to make a test object.
    fn make_to(uid: &str) -> TO {
        WatchResponse::Ok(WatchEvent::Added(Pod {
            metadata: ObjectMeta {
                uid: Some(uid.to_owned()),
                ..ObjectMeta::default()
            },
            ..Pod::default()
        }))
    }

    fn assert_test_object(tested_test_object: Option<Result<TO, Error>>, expected_uid: &str) {
        let actual_to = tested_test_object
            .expect("expected an yielded entry, but none found")
            .expect("parsing failed");
        let expected_to = make_to(expected_uid);
        match (actual_to, expected_to) {
            (WatchResponse::Ok(actual_event), WatchResponse::Ok(expected_event)) => {
                assert_eq!(actual_event, expected_event)
            }
            _ => panic!("expected an event, got something else"),
        }
    }

    fn make_state<T>() -> crate::State<Decoder<T>, Vec<u8>> {
        crate::State {
            buffer: Vec::new(),
            decoder: Decoder::new(),
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
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid0"
                        }
                    }
                }"#,
            );
            assert_test_object(stream.next(), "uid0");
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
                    "type": "ADDED",
                    "ob"#,
            );
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"ject": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid0"
                        }
                    }
                }"#,
            );
            assert_test_object(stream.next(), "uid0");
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
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid0"
                        }
                    }
                }{
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid1"
                        }
                    }
                }"#,
            );
            assert_test_object(stream.next(), "uid0");
            assert_test_object(stream.next(), "uid1");
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
                    "type": "ADDED",
                    "ob"#,
            );
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"ject": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid0"
                        }
                    }
                }{
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVe"#,
            );
            assert_test_object(stream.next(), "uid0");
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"rsion": "v1",
                        "metadata": {
                            "uid": "uid1"
                        }
                    }
                }"#,
            );
            assert_test_object(stream.next(), "uid1");
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
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid0"
                        }
                    }
                }{
                    "type": "ADDED",
                    "ob"#,
            );
            assert_test_object(stream.next(), "uid0");
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"ject": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid1"
                        }
                    }
                }{
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid2"
                        }
                    }
                }{
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVe"#,
            );
            assert_test_object(stream.next(), "uid1");
            assert_test_object(stream.next(), "uid2");
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"rsion": "v1",
                        "metadata": {
                            "uid": "uid3"
                        }
                    }
                }{
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid4"
                        }
                    }
                }"#,
            );
            assert_test_object(stream.next(), "uid3");
            assert_test_object(stream.next(), "uid4");
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
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid0"
                        }
                    }
                }"#,
            );
            assert_test_object(stream.next(), "uid0");
            assert!(stream.next().is_none());
        }

        {
            let mut stream = dec.process_next_chunk(
                br#"{
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid1"
                        }
                    }
                }"#,
            );
            assert_test_object(stream.next(), "uid1");
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
                    "type": "ADDED",
                    "object": {
                        "kind": "Pod",
                        "apiVersion": "v1",
                        "metadata": {
                            "uid": "uid0"
                        }
                    }
                }{"#,
            );
            assert_test_object(stream.next(), "uid0");
            assert!(stream.next().is_none());
        }

        assert_eq!(dec.finish().unwrap_err(), b"{");
    }

    #[test]
    fn test_allows_unparsed_newlines_at_finish() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(b"\n");
            assert!(stream.next().is_none());
        }

        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_allows_two_unparsed_newlines_at_finish() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(b"\n\n");
            assert!(stream.next().is_none());
        }

        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_allows_three_unparsed_newlines_at_finish() {
        let mut dec = make_state::<TO>();

        {
            let mut stream = dec.process_next_chunk(b"\n\n\n");
            assert!(stream.next().is_none());
        }

        assert!(dec.finish().is_ok());
    }

    #[test]
    fn test_memory_usage() {
        let mut dec = make_state::<TO>();

        let chunk = br#"{
            "type": "ADDED",
            "object": {
                "kind": "Pod",
                "apiVersion": "v1",
                "metadata": {
                    "uid": "uid0"
                }
            }
        }"#;
        let mut chunks = chunk.iter().cycle();

        let max_chunks_per_iter = 15;

        // Simulate processing a huge number of items.
        for _ in 0..100_000 {
            // Take random amout of bytes from the chunks iter and prepare the
            // next chunk.
            let to_take = rand::random::<usize>() % (chunk.len() * max_chunks_per_iter);
            let next_chunk = (&mut chunks).take(to_take).cloned().collect::<Box<_>>();

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

    #[test]
    fn test_practical_error_case_1() {
        let mut dec = make_state::<TO>();

        let sample = b"{\"type\":\"BOOKMARK\",\"object\":{\"kind\":\"Pod\",\"apiVersion\":\"v1\",\"metadata\":{\"resourceVersion\":\"3845\",\"creationTimestamp\":null},\"spec\":{\"containers\":null},\"status\":{}}}\n";

        {
            let mut stream = dec.process_next_chunk(sample);
            let actual_to = stream
                .next()
                .expect("expected an yielded entry, but none found")
                .expect("parsing failed");
            let expected_event = WatchEvent::Bookmark {
                resource_version: "3845".into(),
            };
            match actual_to {
                WatchResponse::Ok(actual_event) => assert_eq!(actual_event, expected_event),
                _ => panic!("expected an event, got something else"),
            }
        }

        assert!(dec.finish().is_ok());
    }
}
