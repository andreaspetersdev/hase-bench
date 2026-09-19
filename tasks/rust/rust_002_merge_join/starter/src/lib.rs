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
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl<L, R, F> FusedIterator for MergeJoinBy<L, R, F>
where
    L: Iterator + FusedIterator,
    R: Iterator + FusedIterator,
    F: FnMut(&L::Item, &R::Item) -> Ordering,
{
}
