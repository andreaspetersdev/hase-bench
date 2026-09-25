use std::fmt;
use std::io::{self, Read, Write};

pub const MINIMUM_PROTOCOL: u32 = 31;
pub const MAXIMUM_PEER_PROTOCOL: u32 = 40;
pub const MULTIPLEX_BASE: u8 = 7;
pub const MAXIMUM_MULTIPLEX_PAYLOAD: usize = 0x00ff_ffff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Data(Vec<u8>),
    Error(String),
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedProtocol {
    pub version: u32,
    pub checksum: String,
    pub compression: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MultiplexCode {
    Data = 0,
    ErrorTransfer = 1,
    Info = 2,
    Error = 3,
    Warning = 4,
    ErrorSocket = 5,
    Log = 6,
    Client = 7,
    ErrorUtf8 = 8,
    Redo = 9,
    Stats = 10,
    IoError = 22,
    IoTimeout = 33,
    Noop = 42,
    ErrorExit = 86,
    Success = 100,
    Deleted = 101,
    NoSend = 102,
}

impl MultiplexCode {
    pub const ALL: [Self; 18] = [
        Self::Data,
        Self::ErrorTransfer,
        Self::Info,
        Self::Error,
        Self::Warning,
        Self::ErrorSocket,
        Self::Log,
        Self::Client,
        Self::ErrorUtf8,
        Self::Redo,
        Self::Stats,
        Self::IoError,
        Self::IoTimeout,
        Self::Noop,
        Self::ErrorExit,
        Self::Success,
        Self::Deleted,
        Self::NoSend,
    ];

    pub fn wire_tag(self) -> u8 {
        MULTIPLEX_BASE + self as u8
    }

    fn from_wire_tag(tag: u8) -> Result<Self, WireError> {
        let Some(code) = tag.checked_sub(MULTIPLEX_BASE) else {
            return Err(WireError::InvalidMultiplexTag(tag));
        };
        match code {
            0 => Ok(Self::Data),
            1 => Ok(Self::ErrorTransfer),
            2 => Ok(Self::Info),
            3 => Ok(Self::Error),
            4 => Ok(Self::Warning),
            5 => Ok(Self::ErrorSocket),
            6 => Ok(Self::Log),
            7 => Ok(Self::Client),
            8 => Ok(Self::ErrorUtf8),
            9 => Ok(Self::Redo),
            10 => Ok(Self::Stats),
            22 => Ok(Self::IoError),
            33 => Ok(Self::IoTimeout),
            42 => Ok(Self::Noop),
            86 => Ok(Self::ErrorExit),
            100 => Ok(Self::Success),
            101 => Ok(Self::Deleted),
            102 => Ok(Self::NoSend),
            _ => Err(WireError::InvalidMultiplexTag(tag)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiplexFrame {
    pub code: MultiplexCode,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum WireError {
    Io(io::Error),
    InvalidProtocolVersion(u32),
    UnsupportedProtocolVersion(u32),
    InvalidMultiplexTag(u8),
    FrameTooLarge { length: usize, limit: usize },
}

impl fmt::Display for WireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "wire I/O failed: {error}"),
            Self::InvalidProtocolVersion(version) => {
                write!(formatter, "invalid peer protocol version {version}")
            }
            Self::UnsupportedProtocolVersion(version) => write!(
                formatter,
                "peer protocol version {version} is below required version {MINIMUM_PROTOCOL}"
            ),
            Self::InvalidMultiplexTag(tag) => {
                write!(formatter, "invalid rsync multiplex tag {tag}")
            }
            Self::FrameTooLarge { length, limit } => {
                write!(
                    formatter,
                    "multiplex payload length {length} exceeds limit {limit}"
                )
            }
        }
    }
}

impl std::error::Error for WireError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for WireError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn negotiate_protocol_version(remote_version: u32) -> Result<u32, WireError> {
    validate_protocol_version(remote_version)?;
    Ok(MINIMUM_PROTOCOL.min(remote_version))
}

pub fn read_protocol_version<R: Read>(reader: &mut R) -> Result<u32, WireError> {
    let mut encoded = [0_u8; 4];
    reader.read_exact(&mut encoded)?;
    let version = u32::from_le_bytes(encoded);
    validate_protocol_version(version)?;
    Ok(version)
}

pub fn write_protocol_version<W: Write>(writer: &mut W) -> Result<(), WireError> {
    writer.write_all(&MINIMUM_PROTOCOL.to_le_bytes())?;
    Ok(())
}

pub fn read_multiplex_frame<R: Read>(
    reader: &mut R,
    payload_limit: usize,
) -> Result<MultiplexFrame, WireError> {
    let mut encoded = [0_u8; 4];
    reader.read_exact(&mut encoded)?;
    let header = u32::from_le_bytes(encoded);
    let length = (header & 0x00ff_ffff) as usize;
    let tag = (header >> 24) as u8;
    let code = MultiplexCode::from_wire_tag(tag)?;
    let effective_limit = payload_limit.min(MAXIMUM_MULTIPLEX_PAYLOAD);
    if length > effective_limit {
        return Err(WireError::FrameTooLarge {
            length,
            limit: effective_limit,
        });
    }
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload)?;
    Ok(MultiplexFrame { code, payload })
}

pub fn write_multiplex_frame<W: Write>(
    writer: &mut W,
    frame: &MultiplexFrame,
    payload_limit: usize,
) -> Result<(), WireError> {
    let effective_limit = payload_limit.min(MAXIMUM_MULTIPLEX_PAYLOAD);
    if frame.payload.len() > effective_limit {
        return Err(WireError::FrameTooLarge {
            length: frame.payload.len(),
            limit: effective_limit,
        });
    }
    let header = ((frame.code.wire_tag() as u32) << 24) | frame.payload.len() as u32;
    writer.write_all(&header.to_le_bytes())?;
    writer.write_all(&frame.payload)?;
    Ok(())
}

fn validate_protocol_version(version: u32) -> Result<(), WireError> {
    if version < MINIMUM_PROTOCOL {
        Err(WireError::UnsupportedProtocolVersion(version))
    } else if version > MAXIMUM_PEER_PROTOCOL {
        Err(WireError::InvalidProtocolVersion(version))
    } else {
        Ok(())
    }
}
