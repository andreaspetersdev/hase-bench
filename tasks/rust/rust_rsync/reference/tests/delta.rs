use rust_rsync::delta::Instruction;
use rust_rsync_reference::delta::{
    DeltaError, create_plan, literal_bytes, reconstruct, signatures,
};

#[test]
fn rolling_and_strong_signatures_reconstruct_exact_target() {
    let basis = (0_u32..4096).flat_map(u32::to_le_bytes).collect::<Vec<_>>();
    let mut target = basis.clone();
    target.splice(1536..1544, b"inserted-delta".iter().copied());
    target.drain(9000..9021);
    let plan = create_plan(&basis, &target, 256).unwrap();
    let rebuilt = reconstruct(&basis, &plan, target.len() as u64).unwrap();
    assert_eq!(rebuilt, target);
    assert!(
        plan.instructions
            .iter()
            .any(|item| matches!(item, Instruction::Match { .. }))
    );
}

#[test]
fn localized_edits_reuse_blocks_and_bound_literal_transfer() {
    let basis = vec![b'a'; 16 * 1024];
    let mut target = basis.clone();
    target[4097..4105].copy_from_slice(b"changed!");
    let plan = create_plan(&basis, &target, 512).unwrap();
    assert!(literal_bytes(&plan) < 1024);
    assert_eq!(reconstruct(&basis, &plan, 16 * 1024).unwrap(), target);
}

#[test]
fn reconstruction_rejects_corruption_invalid_indices_and_limits() {
    let basis = b"abcdefghabcdefgh";
    let target = b"abcdEFGHabcdefgh";
    let plan = create_plan(basis, target, 8).unwrap();
    assert_eq!(
        reconstruct(basis, &plan, target.len() as u64 - 1),
        Err(DeltaError::OutputLimitExceeded)
    );

    let mut corrupt = plan.clone();
    if let Some(Instruction::Literal(bytes)) = corrupt
        .instructions
        .iter_mut()
        .find(|item| matches!(item, Instruction::Literal(_)))
    {
        bytes[0] ^= 0xff;
    }
    assert_eq!(
        reconstruct(basis, &corrupt, target.len() as u64),
        Err(DeltaError::OutputDigestMismatch)
    );

    let mut invalid = plan;
    invalid.instructions = vec![Instruction::Match {
        block_index: u64::MAX,
    }];
    assert_eq!(
        reconstruct(basis, &invalid, target.len() as u64),
        Err(DeltaError::InvalidBlockIndex(u64::MAX))
    );
}

#[test]
fn signature_configuration_rejects_zero_block_size() {
    assert_eq!(signatures(b"basis", 0), Err(DeltaError::InvalidBlockSize));
    assert_eq!(
        create_plan(b"basis", b"target", 0),
        Err(DeltaError::InvalidBlockSize)
    );
}
