//! Property tests for Web compatibility modules.
//!
//! This file contains property-based tests for the dx-compat-web crate,
//! validating correctness properties defined in the design document.

#[cfg(feature = "web-core")]
mod tests {
    use dx_js_compatibility::web::{fetch, streams};
    use proptest::prelude::*;

    // =========================================================================
    // Property 15: Fetch Response Body Consistency
    // Validates: Requirements 10.1, 10.2, 10.3
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 15: Headers are case-insensitive.
        #[test]
        fn headers_case_insensitive(
            name in "[a-zA-Z][a-zA-Z0-9-]{0,20}",
            value in "[a-zA-Z0-9 ]{1,50}"
        ) {
            let mut headers = fetch::Headers::new();
            headers.set(&name, &value);

            let lower = name.to_lowercase();
            let upper = name.to_uppercase();

            prop_assert_eq!(
                headers.get(&lower),
                Some(value.as_str()),
                "Headers should be case-insensitive (lowercase)"
            );
            prop_assert_eq!(
                headers.get(&upper),
                Some(value.as_str()),
                "Headers should be case-insensitive (uppercase)"
            );
        }

        /// Property 15: Headers.delete() removes header.
        #[test]
        fn headers_delete_removes(
            name in "[a-zA-Z][a-zA-Z0-9-]{0,20}",
            value in "[a-zA-Z0-9 ]{1,50}"
        ) {
            let mut headers = fetch::Headers::new();
            headers.set(&name, &value);

            prop_assert!(headers.has(&name));

            headers.delete(&name);

            prop_assert!(!headers.has(&name), "Header should be removed after delete");
        }

        /// Property 15: Headers.set() replaces existing value.
        #[test]
        fn headers_set_replaces(
            name in "[a-zA-Z][a-zA-Z0-9-]{0,20}",
            value1 in "[a-zA-Z0-9]{1,20}",
            value2 in "[a-zA-Z0-9]{1,20}"
        ) {
            let mut headers = fetch::Headers::new();
            headers.set(&name, &value1);
            headers.set(&name, &value2);

            prop_assert_eq!(
                headers.get(&name),
                Some(value2.as_str()),
                "set should replace existing value"
            );
        }
    }

    // =========================================================================
    // Property 9: Web Stream Pipe Completeness
    // Validates: Requirements 11.1, 11.4
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 9: pipeTo() SHALL transfer all chunks in order.
        #[test]
        fn web_stream_pipe_transfers_all_chunks(
            chunks in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 1..100),
                1..20
            )
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                use bytes::Bytes;

                let chunk_bytes: Vec<Bytes> = chunks.iter()
                    .map(|c| Bytes::from(c.clone()))
                    .collect();
                let mut readable = streams::ReadableStream::new(chunk_bytes.clone());
                let mut writable = streams::WritableStream::new();

                readable.pipe_to(&mut writable).await.unwrap();

                let written_chunks = writable.chunks();
                prop_assert_eq!(
                    written_chunks.len(),
                    chunks.len(),
                    "Number of written chunks should equal number of source chunks"
                );

                for (i, (written, original)) in written_chunks.iter().zip(chunk_bytes.iter()).enumerate() {
                    prop_assert_eq!(&written[..], &original[..], "Chunk {} content should match", i);
                }
                Ok(())
            })?;
        }

        /// Property 9: Total bytes written SHALL equal total bytes read.
        #[test]
        fn web_stream_pipe_preserves_total_bytes(
            chunks in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 0..200),
                0..30
            )
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                use bytes::Bytes;

                let total_bytes: usize = chunks.iter().map(|c| c.len()).sum();
                let chunk_bytes: Vec<Bytes> = chunks.iter()
                    .map(|c| Bytes::from(c.clone()))
                    .collect();
                let mut readable = streams::ReadableStream::new(chunk_bytes);
                let mut writable = streams::WritableStream::new();

                readable.pipe_to(&mut writable).await.unwrap();

                let written_bytes: usize = writable.chunks().iter().map(|c| c.len()).sum();
                prop_assert_eq!(
                    written_bytes,
                    total_bytes,
                    "Total written bytes should equal total source bytes"
                );
                Ok(())
            })?;
        }

        /// Property 9: tee() SHALL create two streams with identical content.
        #[test]
        fn readable_stream_tee_creates_copies(
            chunks in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 1..50),
                1..10
            )
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                use bytes::Bytes;

                let chunk_bytes: Vec<Bytes> = chunks.iter()
                    .map(|c| Bytes::from(c.clone()))
                    .collect();
                let readable = streams::ReadableStream::new(chunk_bytes.clone());

                let (stream1, stream2) = readable.tee();
                let mut reader1 = stream1.get_reader();
                let mut reader2 = stream2.get_reader();

                let mut chunks1 = Vec::new();
                let mut chunks2 = Vec::new();

                while let Some(chunk) = reader1.read().await {
                    chunks1.push(chunk);
                }
                while let Some(chunk) = reader2.read().await {
                    chunks2.push(chunk);
                }

                prop_assert_eq!(chunks1.len(), chunks.len());
                prop_assert_eq!(chunks2.len(), chunks.len());

                for (i, ((c1, c2), original)) in chunks1.iter().zip(chunks2.iter()).zip(chunk_bytes.iter()).enumerate() {
                    prop_assert_eq!(&c1[..], &original[..], "Stream1 chunk {} should match", i);
                    prop_assert_eq!(&c2[..], &original[..], "Stream2 chunk {} should match", i);
                }
                Ok(())
            })?;
        }
    }
}
