#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSignature {
    pub index: u64,
    pub length: u32,
    pub weak: u32,
    pub strong: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Match { block_index: u64 },
    Literal(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaPlan {
    pub block_size: u32,
    pub target_size: u64,
    pub target_digest: Vec<u8>,
    pub instructions: Vec<Instruction>,
}
