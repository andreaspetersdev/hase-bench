const HEADER_LEN: usize = 8;
const TRAILER_LEN: usize = 4;
const FNV_OFFSET: u32 = 2_166_136_261;
const FNV_PRIME: u32 = 16_777_619;

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

enum State {
    Active,
    Finished,
    Failed(ParseError),
}

pub struct FrameParser {
    max_payload: usize,
    buffer: Vec<u8>,
    frames: Vec<Frame>,
    state: State,
}

impl FrameParser {
    pub fn new(max_payload: usize) -> Self {
        Self {
            max_payload,
            buffer: Vec::new(),
            frames: Vec::new(),
            state: State::Active,
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<(), ParseError> {
        match &self.state {
            State::Failed(error) => return Err(error.clone()),
            State::Finished => return Err(ParseError::AlreadyFinished),
            State::Active => {}
        }
        self.buffer.extend_from_slice(chunk);
        self.parse_available()
    }

    pub fn finish(&mut self) -> Result<(), ParseError> {
        match &self.state {
            State::Failed(error) => return Err(error.clone()),
            State::Finished => return Ok(()),
            State::Active => {}
        }
        self.parse_available()?;
        if self.buffer.is_empty() {
            self.state = State::Finished;
            return Ok(());
        }
        let needed = if self.buffer.len() < HEADER_LEN {
            HEADER_LEN
        } else {
            self.frame_size()
        };
        self.fail(ParseError::Truncated {
            buffered: self.buffer.len(),
            needed,
        })
    }

    pub fn take_frames(&mut self) -> Vec<Frame> {
        std::mem::take(&mut self.frames)
    }

    pub fn status(&self) -> ParserStatus {
        match self.state {
            State::Active => ParserStatus::Active,
            State::Finished => ParserStatus::Finished,
            State::Failed(_) => ParserStatus::Failed,
        }
    }

    pub fn error(&self) -> Option<&ParseError> {
        match &self.state {
            State::Failed(error) => Some(error),
            State::Active | State::Finished => None,
        }
    }

    fn parse_available(&mut self) -> Result<(), ParseError> {
        loop {
            if let Some(&found) = self.buffer.first()
                && found != b'H'
            {
                return self.fail(ParseError::InvalidMagic { index: 0, found });
            }
            if let Some(&found) = self.buffer.get(1)
                && found != b'B'
            {
                return self.fail(ParseError::InvalidMagic { index: 1, found });
            }
            if let Some(&version) = self.buffer.get(2)
                && version != 1
            {
                return self.fail(ParseError::UnsupportedVersion(version));
            }
            if self.buffer.len() < HEADER_LEN {
                return Ok(());
            }
            let length = self.payload_length();
            if u64::from(length) > self.max_payload as u64 {
                return self.fail(ParseError::FrameTooLarge {
                    length,
                    max_payload: self.max_payload,
                });
            }
            let frame_size = self.frame_size();
            if self.buffer.len() < frame_size {
                return Ok(());
            }
            let payload_end = HEADER_LEN + length as usize;
            let expected = checksum(&self.buffer[2..payload_end]);
            let actual = u32::from_be_bytes(
                self.buffer[payload_end..frame_size]
                    .try_into()
                    .expect("checksum span has four bytes"),
            );
            if expected != actual {
                return self.fail(ParseError::ChecksumMismatch { expected, actual });
            }
            self.frames.push(Frame {
                kind: self.buffer[3],
                payload: self.buffer[HEADER_LEN..payload_end].to_vec(),
            });
            self.buffer.drain(..frame_size);
        }
    }

    fn payload_length(&self) -> u32 {
        u32::from_be_bytes(
            self.buffer[4..HEADER_LEN]
                .try_into()
                .expect("complete header has length bytes"),
        )
    }

    fn frame_size(&self) -> usize {
        HEADER_LEN + self.payload_length() as usize + TRAILER_LEN
    }

    fn fail<T>(&mut self, error: ParseError) -> Result<T, ParseError> {
        self.state = State::Failed(error.clone());
        Err(error)
    }
}

fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(FNV_OFFSET, |value, byte| {
        (value ^ u32::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}
