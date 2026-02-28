//! Property-based tests for HBTP IPC

use dx_py_ipc::{
    protocol::ArrayDtype, protocol::ArrayMetadata, HbtpChannel, HbtpFlags, HbtpHeader, MessageType,
    SharedArrayHandle,
};
use proptest::prelude::*;

/// Property 3: HBTP Serialization Round-Trip
mod roundtrip_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Header serialization is reversible
        #[test]
        fn prop_header_roundtrip(
            msg_type in 0u8..10,
            flags in 0u8..64,
            payload_len in any::<u32>()
        ) {
            let msg_type = MessageType::from_u8(msg_type).unwrap_or(MessageType::Ping);
            let flags = HbtpFlags::from_bits(flags).unwrap_or(HbtpFlags::empty());

            let header = HbtpHeader::new(msg_type, flags, payload_len);
            let bytes = header.to_bytes();
            let restored = HbtpHeader::from_bytes(&bytes).unwrap();

            prop_assert_eq!(restored.msg_type, msg_type);
            prop_assert_eq!(restored.flags, flags);
            prop_assert_eq!(restored.payload_len, payload_len);
        }

        /// Message serialization is reversible
        #[test]
        fn prop_message_roundtrip(
            payload in prop::collection::vec(any::<u8>(), 0..1000)
        ) {
            use dx_py_ipc::channel::HbtpMessage;

            let message = HbtpMessage::new(
                MessageType::TransferObject,
                HbtpFlags::REQUIRES_ACK,
                payload.clone(),
            );

            let bytes = message.to_bytes();
            let restored = HbtpMessage::from_bytes(&bytes).unwrap();

            prop_assert_eq!(restored.header.msg_type, MessageType::TransferObject);
            prop_assert!(restored.header.flags.contains(HbtpFlags::REQUIRES_ACK));
            prop_assert_eq!(restored.payload, payload);
        }

        /// SharedArrayHandle serialization is reversible
        #[test]
        fn prop_array_handle_roundtrip(
            arena_name in "[a-z]{1,10}",
            offset in any::<usize>(),
            dtype in 0u8..13,
            shape in prop::collection::vec(1u64..100, 1..4)
        ) {
            let dtype = ArrayDtype::from_u8(dtype).unwrap_or(ArrayDtype::Float64);
            let metadata = ArrayMetadata::new(dtype, &shape);

            let handle = SharedArrayHandle {
                arena_name: arena_name.clone(),
                offset,
                metadata,
            };

            let bytes = handle.to_bytes();
            let restored = SharedArrayHandle::from_bytes(&bytes).unwrap();

            prop_assert_eq!(restored.arena_name, arena_name);
            prop_assert_eq!(restored.offset, offset);
            prop_assert_eq!(restored.metadata.dtype, dtype);
            prop_assert_eq!(restored.metadata.ndim, shape.len() as u8);
        }
    }
}

/// Tests for array transfer
mod array_transfer_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Array data is preserved through channel transfer
        #[test]
        fn prop_array_data_preserved(
            data in prop::collection::vec(any::<u8>(), 1..500)
        ) {
            let channel = HbtpChannel::new();
            let metadata = ArrayMetadata::new(ArrayDtype::UInt8, &[data.len() as u64]);

            channel.send_array(&data, metadata).unwrap();

            // Move from send to recv for testing
            while let Some(msg) = channel.send_queue().pop() {
                channel.recv_queue().push(msg);
            }

            let (recv_data, recv_meta) = channel.recv_array().unwrap().unwrap();

            prop_assert_eq!(recv_data, data.clone());
            prop_assert_eq!(recv_meta.size as usize, data.len());
        }

        /// Array metadata is preserved
        #[test]
        fn prop_array_metadata_preserved(
            shape in prop::collection::vec(1u64..50, 1..3)
        ) {
            let channel = HbtpChannel::new();
            let size: u64 = shape.iter().product();
            let data: Vec<u8> = (0..size as usize).map(|i| i as u8).collect();
            let metadata = ArrayMetadata::new(ArrayDtype::UInt8, &shape);

            channel.send_array(&data, metadata.clone()).unwrap();

            while let Some(msg) = channel.send_queue().pop() {
                channel.recv_queue().push(msg);
            }

            let (_, recv_meta) = channel.recv_array().unwrap().unwrap();

            prop_assert_eq!(recv_meta.ndim, shape.len() as u8);
            for (i, &expected) in shape.iter().enumerate() {
                prop_assert_eq!(recv_meta.shape[i], expected);
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_header_magic() {
        let header = HbtpHeader::new(MessageType::Ping, HbtpFlags::empty(), 0);
        assert!(header.validate_magic());
        // Access packed field through a copy
        let magic = header.magic;
        assert_eq!(magic, 0xDEAD);
    }

    #[test]
    fn test_all_message_types() {
        for i in 0..10u8 {
            let msg_type = MessageType::from_u8(i);
            assert!(msg_type.is_some(), "Message type {} should be valid", i);
        }
        assert!(MessageType::from_u8(100).is_none());
    }

    #[test]
    fn test_all_dtypes() {
        for i in 0..13u8 {
            let dtype = ArrayDtype::from_u8(i);
            assert!(dtype.is_some(), "Dtype {} should be valid", i);
        }
        assert!(ArrayDtype::from_u8(100).is_none());
    }

    #[test]
    fn test_dtype_sizes() {
        assert_eq!(ArrayDtype::Int8.size(), 1);
        assert_eq!(ArrayDtype::Int16.size(), 2);
        assert_eq!(ArrayDtype::Int32.size(), 4);
        assert_eq!(ArrayDtype::Int64.size(), 8);
        assert_eq!(ArrayDtype::Float32.size(), 4);
        assert_eq!(ArrayDtype::Float64.size(), 8);
        assert_eq!(ArrayDtype::Complex128.size(), 16);
    }

    #[test]
    fn test_channel_close() {
        let channel = HbtpChannel::new();
        assert!(!channel.is_closed());

        channel.close();
        assert!(channel.is_closed());

        // Sending should fail after close
        use dx_py_ipc::channel::HbtpMessage;
        let result = channel.send(HbtpMessage::ping());
        assert!(result.is_err());
    }
}
