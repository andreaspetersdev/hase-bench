use std::cmp::Ordering;
use std::iter::{FusedIterator, Peekable};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeItem<L, R> {
    Left(L),
    Right(R),
    Both(L, R),
}

pub struct MergeJoinBy<L: Iterator, R: Iterator, F> {
    left: Peekable<L>,
    right: Peekable<R>,
    compare: F,
}

pub fn merge_join_by<L, R, F>(
    left: L,
    right: R,
    compare: F,
) -> MergeJoinBy<L::IntoIter, R::IntoIter, F>
where
    L: IntoIterator,
    R: IntoIterator,
    F: FnMut(&L::Item, &R::Item) -> Ordering,
{
    MergeJoinBy {
        left: left.into_iter().peekable(),
        right: right.into_iter().peekable(),
        compare,
    }
}

impl<L, R, F> Iterator for MergeJoinBy<L, R, F>
where
    L: Iterator,
    R: Iterator,
    F: FnMut(&L::Item, &R::Item) -> Ordering,
{
    type Item = MergeItem<L::Item, R::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.left.peek(), self.right.peek()) {
            (None, None) => None,
            (Some(_), None) => self.left.next().map(MergeItem::Left),
            (None, Some(_)) => self.right.next().map(MergeItem::Right),
            (Some(left), Some(right)) => match (self.compare)(left, right) {
                Ordering::Less => self.left.next().map(MergeItem::Left),
                Ordering::Greater => self.right.next().map(MergeItem::Right),
                Ordering::Equal => Some(MergeItem::Both(
                    self.left.next().expect("peeked left item"),
                    self.right.next().expect("peeked right item"),
                )),
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (left_lower, left_upper) = self.left.size_hint();
        let (right_lower, right_upper) = self.right.size_hint();
        let upper = left_upper
            .zip(right_upper)
            .and_then(|(left, right)| left.checked_add(right));
        (left_lower.max(right_lower), upper)
    }
}

impl<L, R, F> FusedIterator for MergeJoinBy<L, R, F>
where
    L: Iterator + FusedIterator,
    R: Iterator + FusedIterator,
    F: FnMut(&L::Item, &R::Item) -> Ordering,
{
}
