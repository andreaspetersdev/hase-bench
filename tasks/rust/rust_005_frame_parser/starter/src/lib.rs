#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub kind: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidMagic { index: u8, found: u8 },
    UnsupportedVersion(u8),
    FrameTooLarge { length: u32, max_payload: usize },
    ChecksumMismatch { expected: u32, actual: u32 },
    Truncated { buffered: usize, needed: usize },
    AlreadyFinished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserStatus {
    Active,
    Finished,
    Failed,
}

pub struct FrameParser {
    max_payload: usize,
    frames: Vec<Frame>,
}

impl FrameParser {
    pub fn new(max_payload: usize) -> Self {
        Self {
            max_payload,
            frames: Vec::new(),
        }
    }

    pub fn push(&mut self, _chunk: &[u8]) -> Result<(), ParseError> {
        let _ = self.max_payload;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), ParseError> {
        Ok(())
    }

    pub fn take_frames(&mut self) -> Vec<Frame> {
        std::mem::take(&mut self.frames)
    }

    pub fn status(&self) -> ParserStatus {
        ParserStatus::Active
    }

    pub fn error(&self) -> Option<&ParseError> {
        None
    }
}
