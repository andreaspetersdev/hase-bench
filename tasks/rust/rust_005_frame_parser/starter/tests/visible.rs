use rust_005::{Frame, FrameParser, ParseError, ParserStatus};

const OFFSET: u32 = 2_166_136_261;
const PRIME: u32 = 16_777_619;

fn encoded(kind: u8, payload: &[u8]) -> Vec<u8> {
    let length = u32::try_from(payload.len()).unwrap();
    let length_bytes = length.to_be_bytes();
    let mut checksum = OFFSET;
    for byte in [1, kind]
        .into_iter()
        .chain(length_bytes)
        .chain(payload.iter().copied())
    {
        checksum ^= u32::from(byte);
        checksum = checksum.wrapping_mul(PRIME);
    }
    let mut bytes = vec![b'H', b'B', 1, kind];
    bytes.extend_from_slice(&length_bytes);
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(&checksum.to_be_bytes());
    bytes
}

#[test]
fn parses_fragmented_frame_with_arbitrary_payload_bytes() {
    let bytes = encoded(7, &[0, 255, b'H', b'B', 0]);
    let mut parser = FrameParser::new(16);
    for byte in bytes {
        parser.push(&[byte]).unwrap();
    }
    parser.finish().unwrap();
    assert_eq!(parser.status(), ParserStatus::Finished);
    assert_eq!(
        parser.take_frames(),
        vec![Frame {
            kind: 7,
            payload: vec![0, 255, b'H', b'B', 0],
        }]
    );
}

#[test]
fn parses_multiple_frames_and_drains_ready_frames() {
    let mut bytes = encoded(1, b"one");
    bytes.extend(encoded(2, b"two"));
    let mut parser = FrameParser::new(3);
    parser.push(&bytes).unwrap();
    assert_eq!(
        parser.take_frames(),
        vec![
            Frame {
                kind: 1,
                payload: b"one".to_vec(),
            },
            Frame {
                kind: 2,
                payload: b"two".to_vec(),
            },
        ]
    );
    assert!(parser.take_frames().is_empty());
}

#[test]
fn reports_size_and_checksum_errors_permanently() {
    let mut oversized = FrameParser::new(2);
    let header = [b'H', b'B', 1, 9, 0, 0, 0, 3];
    let expected = ParseError::FrameTooLarge {
        length: 3,
        max_payload: 2,
    };
    assert_eq!(oversized.push(&header), Err(expected.clone()));
    assert_eq!(oversized.status(), ParserStatus::Failed);
    assert_eq!(oversized.error(), Some(&expected));
    assert_eq!(oversized.push(&[]), Err(expected.clone()));
    assert_eq!(oversized.finish(), Err(expected));

    let mut corrupt = encoded(4, b"ok");
    *corrupt.last_mut().unwrap() ^= 1;
    let mut parser = FrameParser::new(8);
    assert!(matches!(
        parser.push(&corrupt),
        Err(ParseError::ChecksumMismatch { .. })
    ));
}
