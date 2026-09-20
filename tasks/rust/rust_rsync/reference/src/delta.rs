use std::collections::HashMap;
use std::fmt;

use rust_rsync::delta::{BlockSignature, DeltaPlan, Instruction};
use sha2::{Digest, Sha256};

const MODULUS: u32 = 1 << 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaError {
    InvalidBlockSize,
    BasisTooLarge,
    InvalidBlockIndex(u64),
    OutputLimitExceeded,
    OutputSizeMismatch { expected: u64, actual: u64 },
    OutputDigestMismatch,
}

impl fmt::Display for DeltaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBlockSize => formatter.write_str("block size must fit in a nonzero u32"),
            Self::BasisTooLarge => formatter.write_str("basis contains too many blocks"),
            Self::InvalidBlockIndex(index) => {
                write!(formatter, "invalid basis block index: {index}")
            }
            Self::OutputLimitExceeded => {
                formatter.write_str("reconstructed output exceeds its limit")
            }
            Self::OutputSizeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "output size mismatch: expected {expected}, got {actual}"
                )
            }
            Self::OutputDigestMismatch => formatter.write_str("output digest mismatch"),
        }
    }
}

impl std::error::Error for DeltaError {}

pub fn signatures(basis: &[u8], block_size: usize) -> Result<Vec<BlockSignature>, DeltaError> {
    let block_size = valid_block_size(block_size)?;
    basis
        .chunks(block_size as usize)
        .enumerate()
        .map(|(index, block)| {
            Ok(BlockSignature {
                index: u64::try_from(index).map_err(|_| DeltaError::BasisTooLarge)?,
                length: u32::try_from(block.len()).expect("block size is a u32"),
                weak: weak_checksum(block),
                strong: strong_digest(block),
            })
        })
        .collect()
}

pub fn create_plan(
    basis: &[u8],
    target: &[u8],
    block_size: usize,
) -> Result<DeltaPlan, DeltaError> {
    let block_size = valid_block_size(block_size)?;
    let signatures = signatures(basis, block_size as usize)?;
    let mut by_weak = HashMap::<u32, Vec<&BlockSignature>>::new();
    for signature in &signatures {
        by_weak.entry(signature.weak).or_default().push(signature);
    }

    let mut instructions = Vec::new();
    let mut literal = Vec::new();
    let mut position = 0_usize;
    let full_block = block_size as usize;
    let mut rolling = target.get(..full_block).map(RollingChecksum::from_block);

    while target.len().saturating_sub(position) >= full_block {
        let window = &target[position..position + full_block];
        let weak = rolling
            .as_ref()
            .expect("full window has a checksum")
            .value();
        if let Some(block_index) = matching_block(window, weak, &by_weak) {
            flush_literal(&mut instructions, &mut literal);
            instructions.push(Instruction::Match { block_index });
            position += full_block;
            rolling = target
                .get(position..position.saturating_add(full_block))
                .map(RollingChecksum::from_block);
        } else {
            literal.push(target[position]);
            let old = target[position];
            position += 1;
            if position + full_block <= target.len() {
                let new = target[position + full_block - 1];
                rolling
                    .as_mut()
                    .expect("rolling window exists")
                    .roll(old, new);
            } else {
                rolling = None;
            }
        }
    }

    if position < target.len() {
        let tail = &target[position..];
        let weak = weak_checksum(tail);
        if let Some(block_index) = matching_block(tail, weak, &by_weak) {
            flush_literal(&mut instructions, &mut literal);
            instructions.push(Instruction::Match { block_index });
        } else {
            literal.extend_from_slice(tail);
        }
    }
    flush_literal(&mut instructions, &mut literal);

    Ok(DeltaPlan {
        block_size,
        target_size: u64::try_from(target.len()).expect("usize fits in u64 on supported hosts"),
        target_digest: strong_digest(target),
        instructions,
    })
}

pub fn reconstruct(
    basis: &[u8],
    plan: &DeltaPlan,
    output_limit: u64,
) -> Result<Vec<u8>, DeltaError> {
    let block_size = valid_block_size(plan.block_size as usize)? as usize;
    if plan.target_size > output_limit {
        return Err(DeltaError::OutputLimitExceeded);
    }
    let capacity =
        usize::try_from(plan.target_size).map_err(|_| DeltaError::OutputLimitExceeded)?;
    let mut output = Vec::with_capacity(capacity);
    for instruction in &plan.instructions {
        let bytes = match instruction {
            Instruction::Match { block_index } => {
                let index = usize::try_from(*block_index)
                    .map_err(|_| DeltaError::InvalidBlockIndex(*block_index))?;
                let start = index
                    .checked_mul(block_size)
                    .ok_or(DeltaError::InvalidBlockIndex(*block_index))?;
                let end = start
                    .checked_add(block_size)
                    .map(|end| end.min(basis.len()))
                    .ok_or(DeltaError::InvalidBlockIndex(*block_index))?;
                basis
                    .get(start..end)
                    .filter(|block| !block.is_empty())
                    .ok_or(DeltaError::InvalidBlockIndex(*block_index))?
            }
            Instruction::Literal(bytes) => bytes,
        };
        let next_size = output
            .len()
            .checked_add(bytes.len())
            .ok_or(DeltaError::OutputLimitExceeded)?;
        if u64::try_from(next_size).map_err(|_| DeltaError::OutputLimitExceeded)? > output_limit {
            return Err(DeltaError::OutputLimitExceeded);
        }
        output.extend_from_slice(bytes);
    }
    let actual = u64::try_from(output.len()).expect("usize fits in u64 on supported hosts");
    if actual != plan.target_size {
        return Err(DeltaError::OutputSizeMismatch {
            expected: plan.target_size,
            actual,
        });
    }
    if strong_digest(&output) != plan.target_digest {
        return Err(DeltaError::OutputDigestMismatch);
    }
    Ok(output)
}

pub fn literal_bytes(plan: &DeltaPlan) -> u64 {
    plan.instructions
        .iter()
        .filter_map(|instruction| match instruction {
            Instruction::Literal(bytes) => Some(bytes.len() as u64),
            Instruction::Match { .. } => None,
        })
        .sum()
}

fn valid_block_size(block_size: usize) -> Result<u32, DeltaError> {
    u32::try_from(block_size)
        .ok()
        .filter(|size| *size != 0)
        .ok_or(DeltaError::InvalidBlockSize)
}

fn matching_block(
    window: &[u8],
    weak: u32,
    by_weak: &HashMap<u32, Vec<&BlockSignature>>,
) -> Option<u64> {
    let candidates = by_weak.get(&weak)?;
    let strong = strong_digest(window);
    candidates
        .iter()
        .find(|candidate| candidate.length as usize == window.len() && candidate.strong == strong)
        .map(|candidate| candidate.index)
}

fn flush_literal(instructions: &mut Vec<Instruction>, literal: &mut Vec<u8>) {
    if !literal.is_empty() {
        instructions.push(Instruction::Literal(std::mem::take(literal)));
    }
}

fn strong_digest(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn weak_checksum(bytes: &[u8]) -> u32 {
    RollingChecksum::from_block(bytes).value()
}

#[derive(Debug, Clone, Copy)]
struct RollingChecksum {
    a: u32,
    b: u32,
    length: u32,
}

impl RollingChecksum {
    fn from_block(bytes: &[u8]) -> Self {
        let length = u32::try_from(bytes.len()).expect("block size is a u32");
        let mut a = 0_u32;
        let mut b = 0_u32;
        for (index, byte) in bytes.iter().enumerate() {
            a = (a + u32::from(*byte)) % MODULUS;
            let weight = length - u32::try_from(index).expect("index fits block length");
            let weighted = (u64::from(weight) * u64::from(*byte)) % u64::from(MODULUS);
            b = (b + weighted as u32) % MODULUS;
        }
        Self { a, b, length }
    }

    fn roll(&mut self, old: u8, new: u8) {
        self.a = (self.a + MODULUS - u32::from(old) + u32::from(new)) % MODULUS;
        let removed = (u64::from(self.length) * u64::from(old) % u64::from(MODULUS)) as u32;
        self.b = (self.b + MODULUS - removed + self.a) % MODULUS;
    }

    fn value(self) -> u32 {
        (self.b << 16) | self.a
    }
}
