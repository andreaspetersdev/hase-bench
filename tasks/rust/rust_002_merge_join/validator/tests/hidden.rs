use std::cell::Cell;
use std::cmp::Ordering;
use std::iter::FusedIterator;
use std::rc::Rc;

use rust_002::{MergeItem, merge_join_by};

fn require_item_traits<T: std::fmt::Debug + Clone + PartialEq + Eq>() {}

#[test]
fn merge_items_keep_the_required_traits() {
    require_item_traits::<MergeItem<i32, String>>();
}

#[derive(Debug, PartialEq, Eq)]
struct MoveOnly {
    key: i32,
    payload: String,
}

#[test]
fn accepts_move_only_items_and_a_stateful_comparator() {
    let left = vec![
        MoveOnly {
            key: 1,
            payload: "left one".to_owned(),
        },
        MoveOnly {
            key: 4,
            payload: "left four".to_owned(),
        },
    ];
    let right = vec![
        MoveOnly {
            key: 2,
            payload: "right two".to_owned(),
        },
        MoveOnly {
            key: 4,
            payload: "right four".to_owned(),
        },
    ];
    let comparisons = Rc::new(Cell::new(0));
    let observed = Rc::clone(&comparisons);
    let output: Vec<_> = merge_join_by(left, right, move |left, right| {
        observed.set(observed.get() + 1);
        left.key.cmp(&right.key)
    })
    .collect();

    assert_eq!(comparisons.get(), 3);
    assert_eq!(output.len(), 3);
    assert!(matches!(&output[0], MergeItem::Left(item) if item.payload == "left one"));
    assert!(matches!(&output[1], MergeItem::Right(item) if item.payload == "right two"));
    assert!(matches!(
        &output[2],
        MergeItem::Both(left, right)
            if left.payload == "left four" && right.payload == "right four"
    ));
}

struct Counting<I> {
    inner: I,
    calls: Rc<Cell<usize>>,
}

impl<I: Iterator> Iterator for Counting<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.calls.set(self.calls.get() + 1);
        self.inner.next()
    }
}

#[test]
fn construction_is_lazy_and_uses_only_one_item_of_lookahead() {
    let left_calls = Rc::new(Cell::new(0));
    let right_calls = Rc::new(Cell::new(0));
    let left = Counting {
        inner: [1, 3].into_iter(),
        calls: Rc::clone(&left_calls),
    };
    let right = Counting {
        inner: [2, 4].into_iter(),
        calls: Rc::clone(&right_calls),
    };

    let mut joined = merge_join_by(left, right, i32::cmp);
    assert_eq!((left_calls.get(), right_calls.get()), (0, 0));
    assert_eq!(joined.next(), Some(MergeItem::Left(1)));
    assert_eq!((left_calls.get(), right_calls.get()), (1, 1));
    assert_eq!(joined.next(), Some(MergeItem::Right(2)));
    assert_eq!((left_calls.get(), right_calls.get()), (2, 1));
}

#[test]
fn size_hint_tracks_the_valid_output_range() {
    let mut joined = merge_join_by([1, 3, 5], [1, 4], i32::cmp);
    assert_eq!(joined.size_hint(), (3, Some(5)));
    assert_eq!(joined.next(), Some(MergeItem::Both(1, 1)));
    assert_eq!(joined.size_hint(), (2, Some(3)));
    assert_eq!(joined.next(), Some(MergeItem::Left(3)));
    assert_eq!(joined.size_hint(), (1, Some(2)));
}

struct HintOnly {
    upper: Option<usize>,
}

impl Iterator for HintOnly {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.upper)
    }
}

#[test]
fn size_hint_handles_unknown_and_overflowing_upper_bounds() {
    let unknown = merge_join_by(
        HintOnly { upper: None },
        HintOnly { upper: Some(1) },
        i32::cmp,
    );
    assert_eq!(unknown.size_hint(), (0, None));

    let overflowing = merge_join_by(
        HintOnly {
            upper: Some(usize::MAX),
        },
        HintOnly { upper: Some(1) },
        i32::cmp,
    );
    assert_eq!(overflowing.size_hint(), (0, None));
}

fn require_fused<I: FusedIterator>(_iterator: &I) {}

#[test]
fn is_fused_when_both_inputs_are_fused() {
    let mut joined = merge_join_by([1], [1], i32::cmp);
    require_fused(&joined);
    assert_eq!(joined.next(), Some(MergeItem::Both(1, 1)));
    assert_eq!(joined.next(), None);
    assert_eq!(joined.next(), None);
}

#[test]
fn pairs_duplicate_runs_and_never_compares_after_exhaustion() {
    let comparisons = Rc::new(Cell::new(0));
    let observed = Rc::clone(&comparisons);
    let output: Vec<_> = merge_join_by([1, 1], [1, 1, 1, 2], move |left, right| {
        observed.set(observed.get() + 1);
        left.cmp(right)
    })
    .collect();

    assert_eq!(
        output,
        vec![
            MergeItem::Both(1, 1),
            MergeItem::Both(1, 1),
            MergeItem::Right(1),
            MergeItem::Right(2),
        ]
    );
    assert_eq!(comparisons.get(), 2);
}

#[test]
fn comparator_direction_is_left_then_right() {
    let output: Vec<_> = merge_join_by([1], [2], |left, right| {
        if left < right {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    })
    .collect();
    assert_eq!(output, vec![MergeItem::Left(1), MergeItem::Right(2)]);
}
