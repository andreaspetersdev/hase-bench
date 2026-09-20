pub const MINIMUM_PROTOCOL: u32 = 31;

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
