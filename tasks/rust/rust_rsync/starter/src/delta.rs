#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSignature {
    pub index: u64,
    pub weak: u32,
    pub strong: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Match { block_index: u64 },
    Literal(Vec<u8>),
}
