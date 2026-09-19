# RUST-002 — Lazy generic merge-join iterator

**Severity: Medium.** This task combines generic iterator state, move-only
ownership, and precise laziness/size-hint invariants in one component. It does
not require concurrency, I/O, multiple modules, or failure recovery.

Implement the generic iterator adaptor declared in `src/lib.rs`. It merges two
sorted input streams and reports whether each next key came from the left
stream, the right stream, or both.

Use Rust 2024. Keep the crate name `rust_002` and preserve this public API:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeItem<L, R> {
    Left(L),
    Right(R),
    Both(L, R),
}

pub struct MergeJoinBy<L: Iterator, R: Iterator, F> { /* private fields */ }

pub fn merge_join_by<L, R, F>(
    left: L,
    right: R,
    compare: F,
) -> MergeJoinBy<L::IntoIter, R::IntoIter, F>
where
    L: IntoIterator,
    R: IntoIterator,
    F: FnMut(&L::Item, &R::Item) -> std::cmp::Ordering;
```

`MergeJoinBy` must implement `Iterator<Item = MergeItem<L::Item, R::Item>>`
when `F` has the comparator signature above. It must also implement
`std::iter::FusedIterator` when both underlying iterators implement
`FusedIterator`.

## Merge rules

- Both inputs are already sorted in non-decreasing order according to
  `compare`. The adaptor does not need to validate their ordering.
- Compare the next unconsumed left and right values as `compare(left, right)`.
- `Ordering::Less` yields `MergeItem::Left(left)` and consumes only the left
  value.
- `Ordering::Greater` yields `MergeItem::Right(right)` and consumes only the
  right value.
- `Ordering::Equal` yields `MergeItem::Both(left, right)` and consumes one
  value from each stream.
- Duplicates are paired one-for-one. Any excess duplicates are emitted from
  their remaining side.
- After either input is exhausted, emit the other input in order without
  calling the comparator again.

The adaptor must be lazy: construction consumes no input, and iteration may
buffer at most one value from each side. It must not require input items or the
input iterators to implement `Clone`, `Copy`, `Default`, or `Ord`. The
comparator may carry mutable state.

`size_hint` must report a valid lower bound equal to the larger underlying
lower bound, because pairing can reduce the output length no further than that.
Its upper bound is the checked sum of the two underlying upper bounds, or
`None` when either upper bound is unknown or the sum overflows.

Build the crate and run the visible tests with:

```text
cargo test --locked
```
