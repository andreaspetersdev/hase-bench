use std::io::{Cursor, ErrorKind};

use rust_rsync::wire::{
    MultiplexCode, MultiplexFrame, WireError, negotiate_protocol_version, read_multiplex_frame,
    read_protocol_version, write_multiplex_frame, write_protocol_version,
};

#[test]
fn protocol_31_uses_little_endian_version_words() {
    let mut encoded = Vec::new();
    write_protocol_version(&mut encoded).unwrap();
    assert_eq!(encoded, [31, 0, 0, 0]);
    assert_eq!(
        read_protocol_version(&mut Cursor::new(encoded)).unwrap(),
        31
    );
    assert_eq!(negotiate_protocol_version(32).unwrap(), 31);
    assert_eq!(negotiate_protocol_version(31).unwrap(), 31);
}

#[test]
fn protocol_negotiation_rejects_old_and_implausible_peers() {
    for version in 31..=40 {
        assert_eq!(negotiate_protocol_version(version).unwrap(), 31);
    }
    assert!(matches!(
        negotiate_protocol_version(30),
        Err(WireError::UnsupportedProtocolVersion(30))
    ));
    assert!(matches!(
        read_protocol_version(&mut Cursor::new(41_u32.to_le_bytes())),
        Err(WireError::InvalidProtocolVersion(41))
    ));
}

#[test]
fn only_declared_multiplex_tags_are_accepted() {
    let valid_tags = MultiplexCode::ALL.map(MultiplexCode::wire_tag);
    for tag in 0_u8..=u8::MAX {
        let header = (u32::from(tag) << 24).to_le_bytes();
        let result = read_multiplex_frame(&mut Cursor::new(header), 0);
        if valid_tags.contains(&tag) {
            assert!(result.is_ok(), "declared tag {tag} was rejected");
        } else {
            assert!(matches!(
                result,
                Err(WireError::InvalidMultiplexTag(rejected)) if rejected == tag
            ));
        }
    }
}

#[test]
fn multiplex_data_header_matches_upstream_encoding() {
    let frame = MultiplexFrame {
        code: MultiplexCode::Data,
        payload: b"abc".to_vec(),
    };
    let mut encoded = Vec::new();
    write_multiplex_frame(&mut encoded, &frame, 1024).unwrap();
    assert_eq!(encoded, [3, 0, 0, 7, b'a', b'b', b'c']);
    assert_eq!(
        read_multiplex_frame(&mut Cursor::new(encoded), 1024).unwrap(),
        frame
    );
}

#[test]
fn every_supported_multiplex_code_round_trips() {
    for code in MultiplexCode::ALL {
        let frame = MultiplexFrame {
            code,
            payload: vec![code as u8, 0xff],
        };
        let mut encoded = Vec::new();
        write_multiplex_frame(&mut encoded, &frame, 64).unwrap();
        assert_eq!(
            read_multiplex_frame(&mut Cursor::new(encoded), 64).unwrap(),
            frame
        );
    }
}

#[test]
fn oversized_length_is_rejected_before_payload_is_read() {
    let header = ((MultiplexCode::Data.wire_tag() as u32) << 24) | 4096;
    let mut encoded = Cursor::new(header.to_le_bytes());
    assert!(matches!(
        read_multiplex_frame(&mut encoded, 1024),
        Err(WireError::FrameTooLarge {
            length: 4096,
            limit: 1024,
        })
    ));
    assert_eq!(encoded.position(), 4);

    let frame = MultiplexFrame {
        code: MultiplexCode::Data,
        payload: vec![0; 5],
    };
    assert!(matches!(
        write_multiplex_frame(&mut Vec::new(), &frame, 4),
        Err(WireError::FrameTooLarge {
            length: 5,
            limit: 4,
        })
    ));
}

#[test]
fn unknown_tags_and_truncated_frames_fail_cleanly() {
    let invalid_tag = (250_u32 << 24).to_le_bytes();
    assert!(matches!(
        read_multiplex_frame(&mut Cursor::new(invalid_tag), 1024),
        Err(WireError::InvalidMultiplexTag(250))
    ));

    let truncated = [3, 0, 0, 7, b'a'];
    let error = read_multiplex_frame(&mut Cursor::new(truncated), 1024).unwrap_err();
    assert!(matches!(
        error,
        WireError::Io(ref source) if source.kind() == ErrorKind::UnexpectedEof
    ));
}
