use rust_005::{Frame, FrameParser, ParseError, ParserStatus};

const OFFSET: u32 = 2_166_136_261;
const PRIME: u32 = 16_777_619;

fn assert_value_traits<T: std::fmt::Debug + Clone + PartialEq + Eq>() {}
fn assert_copy_traits<T: std::fmt::Debug + Clone + Copy + PartialEq + Eq>() {}

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
fn public_types_keep_the_required_value_traits() {
    assert_value_traits::<Frame>();
    assert_value_traits::<ParseError>();
    assert_copy_traits::<ParserStatus>();
}

#[test]
fn replays_every_single_split_and_every_byte_boundary() {
    let bytes = encoded(231, &[0, 1, 2, 127, 128, 254, 255]);
    let expected = vec![Frame {
        kind: 231,
        payload: vec![0, 1, 2, 127, 128, 254, 255],
    }];
    for split in 0..=bytes.len() {
        let mut parser = FrameParser::new(7);
        parser.push(&bytes[..split]).unwrap();
        parser.push(&bytes[split..]).unwrap();
        parser.finish().unwrap();
        assert_eq!(parser.take_frames(), expected, "split {split}");
    }

    let mut parser = FrameParser::new(7);
    for byte in &bytes {
        parser.push(std::slice::from_ref(byte)).unwrap();
    }
    assert_eq!(parser.take_frames(), expected);
}

#[test]
fn accepts_empty_and_maximum_payloads_and_empty_chunks() {
    let empty = encoded(0, b"");
    let maximum = encoded(255, &[9, 8, 7, 6]);
    let mut parser = FrameParser::new(4);
    parser.push(&[]).unwrap();
    parser.push(&empty).unwrap();
    parser.push(&[]).unwrap();
    parser.push(&maximum).unwrap();
    assert_eq!(
        parser.take_frames(),
        vec![
            Frame {
                kind: 0,
                payload: vec![],
            },
            Frame {
                kind: 255,
                payload: vec![9, 8, 7, 6],
            },
        ]
    );

    let mut zero = FrameParser::new(0);
    zero.push(&empty).unwrap();
    zero.finish().unwrap();
    assert_eq!(zero.take_frames().len(), 1);
}

#[test]
fn validates_header_fields_in_wire_order_and_as_soon_as_available() {
    let mut first = FrameParser::new(usize::MAX);
    assert_eq!(
        first.push(b"X"),
        Err(ParseError::InvalidMagic {
            index: 0,
            found: b'X',
        })
    );

    let mut second = FrameParser::new(usize::MAX);
    assert_eq!(
        second.push(b"HX"),
        Err(ParseError::InvalidMagic {
            index: 1,
            found: b'X',
        })
    );

    let mut version = FrameParser::new(0);
    assert_eq!(
        version.push(&[b'H', b'B', 2, 0, 255, 255, 255, 255]),
        Err(ParseError::UnsupportedVersion(2))
    );

    let mut size = FrameParser::new(3);
    assert_eq!(
        size.push(&[b'H', b'B', 1, 0, 0, 0, 0, 4]),
        Err(ParseError::FrameTooLarge {
            length: 4,
            max_payload: 3,
        })
    );

    let mut adversarial = FrameParser::new(1024);
    assert_eq!(
        adversarial.push(&[b'H', b'B', 1, 0, 255, 255, 255, 255]),
        Err(ParseError::FrameTooLarge {
            length: u32::MAX,
            max_payload: 1024,
        })
    );
}

#[test]
fn checksum_error_reports_calculated_and_encoded_values() {
    let mut bytes = encoded(5, b"abc");
    let actual = 0x0102_0304_u32;
    let end = bytes.len();
    bytes[end - 4..].copy_from_slice(&actual.to_be_bytes());
    let mut calculated = OFFSET;
    for byte in &bytes[2..end - 4] {
        calculated = (calculated ^ u32::from(*byte)).wrapping_mul(PRIME);
    }
    let mut parser = FrameParser::new(3);
    assert_eq!(
        parser.push(&bytes),
        Err(ParseError::ChecksumMismatch {
            expected: calculated,
            actual,
        })
    );
}

#[test]
fn preserves_completed_frames_when_a_later_frame_fails() {
    let good = encoded(1, b"kept");
    let mut bad = encoded(2, b"bad");
    *bad.last_mut().unwrap() ^= 0x80;
    let mut chunk = good;
    chunk.extend(bad);
    let mut parser = FrameParser::new(8);
    let error = parser.push(&chunk).unwrap_err();
    assert!(matches!(error, ParseError::ChecksumMismatch { .. }));
    assert_eq!(
        parser.take_frames(),
        vec![Frame {
            kind: 1,
            payload: b"kept".to_vec(),
        }]
    );
    assert!(parser.take_frames().is_empty());
    assert_eq!(parser.status(), ParserStatus::Failed);
    assert_eq!(parser.error(), Some(&error));
    assert_eq!(parser.push(&encoded(3, b"ignored")), Err(error.clone()));
    assert_eq!(parser.finish(), Err(error));
}

#[test]
fn finish_classifies_partial_header_and_partial_body_exactly() {
    for buffered in 1..8 {
        let mut parser = FrameParser::new(16);
        let header = [b'H', b'B', 1, 3, 0, 0, 0, 5];
        parser.push(&header[..buffered]).unwrap();
        let expected = ParseError::Truncated {
            buffered,
            needed: 8,
        };
        assert_eq!(parser.finish(), Err(expected.clone()));
        assert_eq!(parser.error(), Some(&expected));
        assert_eq!(parser.push(&[]), Err(expected));
    }

    let bytes = encoded(3, b"hello");
    for buffered in 8..bytes.len() {
        let mut parser = FrameParser::new(5);
        parser.push(&bytes[..buffered]).unwrap();
        assert_eq!(
            parser.finish(),
            Err(ParseError::Truncated {
                buffered,
                needed: bytes.len(),
            }),
            "buffered {buffered}"
        );
    }

    let mut combined = encoded(1, b"complete");
    let partial = encoded(2, b"partial");
    combined.extend_from_slice(&partial[..10]);
    let mut parser = FrameParser::new(8);
    parser.push(&combined).unwrap();
    assert_eq!(
        parser.finish(),
        Err(ParseError::Truncated {
            buffered: 10,
            needed: partial.len(),
        })
    );
    assert_eq!(
        parser.take_frames(),
        vec![Frame {
            kind: 1,
            payload: b"complete".to_vec(),
        }]
    );
}

#[test]
fn successful_finish_is_idempotent_and_rejects_all_later_pushes() {
    let mut parser = FrameParser::new(1);
    parser.finish().unwrap();
    parser.finish().unwrap();
    assert_eq!(parser.status(), ParserStatus::Finished);
    assert_eq!(parser.error(), None);
    assert_eq!(parser.push(&[]), Err(ParseError::AlreadyFinished));
    assert_eq!(
        parser.push(&encoded(1, b"x")),
        Err(ParseError::AlreadyFinished)
    );
    assert_eq!(parser.status(), ParserStatus::Finished);
    assert_eq!(parser.error(), None);
}
