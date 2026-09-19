use rust_002::{MergeItem, merge_join_by};

#[test]
fn merges_interleaved_values_and_matches_equal_keys() {
    let output: Vec<_> =
        merge_join_by([1, 3, 5], [2, 3, 4], |left, right| left.cmp(right)).collect();
    assert_eq!(
        output,
        vec![
            MergeItem::Left(1),
            MergeItem::Right(2),
            MergeItem::Both(3, 3),
            MergeItem::Right(4),
            MergeItem::Left(5),
        ]
    );
}

#[test]
fn pairs_duplicates_one_for_one() {
    let output: Vec<_> =
        merge_join_by([1, 1, 1, 2], [1, 1, 3], |left, right| left.cmp(right)).collect();
    assert_eq!(
        output,
        vec![
            MergeItem::Both(1, 1),
            MergeItem::Both(1, 1),
            MergeItem::Left(1),
            MergeItem::Left(2),
            MergeItem::Right(3),
        ]
    );
}

#[test]
fn supports_different_item_types_and_empty_inputs() {
    let left = [(1_u8, "one"), (3, "three")];
    let right: [(u16, bool); 0] = [];
    let output: Vec<_> =
        merge_join_by(left, right, |left, right| left.0.cmp(&(right.0 as u8))).collect();
    assert_eq!(
        output,
        vec![MergeItem::Left((1, "one")), MergeItem::Left((3, "three"))]
    );
}
