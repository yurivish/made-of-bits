use crate::bitvec::BitVec;
use core::marker::PhantomData;
use std::cmp::PartialEq;
use std::collections::VecDeque;
use std::ops::{Range, RangeInclusive};

#[derive(Debug)]
pub(crate) struct Level<V: BitVec> {
    pub(crate) bv: V,
    // the number of zeros at this level (ie. bv.rank0(bv.universe_size())
    pub(crate) nz: u32,
    // unsigned int with a single bit set signifying
    // the magnitude represented at that level.
    // e.g.  levels[0].bit == 1 << levels.len() - 1
    pub(crate) bit: u32,
    // morton mask for this level
    pub(crate) mask: u32,
}

impl<V: BitVec> Level<V> {
    /// Given the start index of a left node on this level, return the split points
    /// that cover the range:
    /// - left is the start of the left node
    /// - mid is the start of the right node
    /// - right is one past the end of the right node
    pub(crate) fn splits(&self, left: u32) -> (u32, u32, u32) {
        (left, left + self.bit, left + self.bit + self.bit)
    }

    // todo: rename to morton_splits and return values similar to morton?
    pub(crate) fn child_symbol_ranges(
        &self,
        left: u32,
        mask: u32,
    ) -> (RangeInclusive<u32>, RangeInclusive<u32>) {
        let (left, mid, right) = self.splits(left);
        (
            mask_range_inclusive(&(left..=mid - 1), mask),
            mask_range_inclusive(&(mid..=right - 1), mask),
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct KeyVal<K, V> {
    pub(crate) k: K,
    pub(crate) v: V,
}

pub(crate) type Val<V> = KeyVal<(), V>;

impl<K, V> From<(K, V)> for KeyVal<K, V> {
    fn from(x: (K, V)) -> Self {
        KeyVal { k: x.0, v: x.1 }
    }
}

/// This trait is implemented for KeyVal values and is used
/// when traversing a level to merge adjacent KeyVals when
/// their keys are the same and the values are deemed to be
/// mergeable by the `mergeable` function in this trait.
// we will merge if the keys are the same and mergeable is true
// later we can figure out if a thing is even Possibly mergeable and if it isn't, skip the mergy logic entirely
pub(crate) trait MaybeMergeable {
    fn mergeable(&self, other: &Self) -> bool {
        false
    }

    fn merge(&mut self, other: Self)
    where
        Self: Sized,
    {
        unimplemented!("must implement `merge` if `mergeable` returns true")
    }
}

// Associate a usize key to an arbitrary value; used for propagating the metadata
// of which original query element a partial query result is associated with as we
// traverse the wavelet tree
impl<K, V: MaybeMergeable> KeyVal<K, V> {
    pub(crate) fn new(key: K, value: V) -> KeyVal<K, V> {
        KeyVal { k: key, v: value }
    }
    pub(crate) fn map<U>(self, f: impl FnOnce(V) -> U) -> KeyVal<K, U> {
        KeyVal {
            k: self.k,
            v: f(self.v),
        }
    }
    // return a new KeyVal with the previous key and new value
    pub(crate) fn val(self, value: V) -> KeyVal<K, V> {
        KeyVal { v: value, ..self }
    }
}

// Helper for traversing the wavelet matrix level by level,
// reusing space when possible and keeping the elements in
// sorted order with respect to the ordering of wavelet tree
// nodes in the wavelet matrix (all left nodes precede all
// right nodes).
#[derive(Debug)]
pub(crate) struct Traversal<K, V> {
    cur: VecDeque<KeyVal<K, V>>,
    next: VecDeque<KeyVal<K, V>>,
    num_left: usize,
}

// Traverse a wavelet matrix levelwise, at each level maintaining tree nodes
// in order they appear in the wavelet matrix (left children preceding right).
//
// The traversal order means that outputs do not appear in the same order as inputs and
// there may be multiple outputs per input (e.g. symbols found within a given index range)
// so associating each batch with an index allows us to track the association between inputs
// and outputs.
// The Key is usually the input index associated with this query, so we can track it through
// the tree.
impl<K: PartialEq, V: MaybeMergeable> Traversal<K, V> {
    pub(crate) fn new(
        keys: impl IntoIterator<Item = K>,
        vals: impl IntoIterator<Item = V>,
    ) -> Self {
        let mut traversal = Self {
            cur: VecDeque::new(),
            next: VecDeque::new(),
            num_left: 0,
        };
        traversal
            .next
            .extend(keys.into_iter().zip(vals.into_iter()).map(Into::into));
        traversal.num_left = 0;
        traversal
    }

    pub(crate) fn traverse(&mut self, mut f: impl FnMut(&[KeyVal<K, V>], &mut Goer<KeyVal<K, V>>)) {
        // precondition: `next` contains things to traverse.
        // postcondition: `next` has the next things to traverse, with (reversed)
        // left children followed by (non-reversed) right children, and num_left
        // indicating the number of left elements.

        // swap next into cur, then clear next
        std::mem::swap(&mut self.cur, &mut self.next);
        self.next.clear();

        // note: rather than reversing the left subtree in advance, here's an idea:
        // we could potentially call the callback twice per level, once with the
        // left iterator reversed, then the right iterator. this gets tricky in terms
        // of the types since the two iterators would be of different types.
        // If we do this, the left slice is cur[..self.num_left] and the right slice
        // is cur[self.num_left..].
        let cur = self.cur.make_contiguous();
        cur[..self.num_left].reverse();

        // for lifetime reasons (to avoid having to pass &mut self into f), create
        // an auxiliary structure to let f recurse left and right.
        let mut go = Goer {
            next: &mut self.next,
            num_left: 0,
            prev_left: None,
            prev_right: None,
        };

        // invoke the traversal function with the current elements and the recursion helper
        // we pass an iterator rather than an element at a time so that f can do its own
        // batching or re-traversal within a single level if it wants to
        f(cur, &mut go);

        go.done();

        // update the number of nodes that went left based on the calls `f` made to `go`
        self.num_left = go.num_left;

        // dbg!(self.next.len());
    }

    pub(crate) fn results(&mut self) -> &mut [KeyVal<K, V>] {
        let slice = self.next.make_contiguous();
        // note: we reverse here so that results are returned in ascending (ie. wm index order along the bottom level).
        slice[..self.num_left].reverse();
        self.num_left = 0; // update this so that calling results multiple times does not re-reverse the left
        slice
    }

    // note: we check whether *next* is empty since that is what will be traversed next, since
    // `next` is swapped into `cur` in `traversal.traverse()`.
    pub(crate) fn is_empty(&self) -> bool {
        self.next.is_empty()
    }
}

/// Passed into the traversal callback as a way to control the recursion.
/// Goes left and/or right.
/// Will merge entries together if they
/// 1. have the same key
/// 2. are mergeable
/// 3. both went in the same direction
/// this is sometimes wasteful since eg. if you're tracking individual symbols
/// down the tree then you cannot merge any entries from the same original query,
/// and could only merge across different input index ranges. But those will have
/// different keys, so they won't be merged here.
/// But merging works well when you don't care about the key -- eg. compute something
/// for a collection of distinct index ranges that are effectively treated as a single
/// discontiguous range. (In this case the key is `()`)
pub(crate) struct Goer<'next, T> {
    // the collection into which we are placing guys
    next: &'next mut VecDeque<T>,
    // number of guys who went left
    num_left: usize,
    // the last seen guy who went left
    prev_left: Option<T>,
    // the last seen guy who went right
    prev_right: Option<T>,
}

impl<T> Goer<'_, T> {
    pub(crate) fn push_left(&mut self, value: T) {
        // left children are appended to the front of the queue,
        // which causes them to be in reverse order, so they get reversed
        // back to the right way round in `traverse` (this is why we track num_left).
        self.next.push_front(value);
        self.num_left += 1;
    }

    pub(crate) fn push_right(&mut self, value: T) {
        // right children are appended to the back of the queue
        self.next.push_back(value);
    }

    pub(crate) fn done(&mut self) {
        if let Some(prev_left) = self.prev_left.take() {
            self.push_left(prev_left);
        }
        if let Some(prev_right) = self.prev_right.take() {
            // right children are appended to the back of the queue
            self.push_right(prev_right);
        }
    }
}

impl<K: PartialEq, V: MaybeMergeable> Goer<'_, KeyVal<K, V>> {
    pub(crate) fn left(&mut self, x: KeyVal<K, V>) {
        match &mut self.prev_left {
            Some(prev_left) => {
                // merge into the previous left entry if the key is the same and the values are mergeable
                if prev_left.k == x.k && V::mergeable(&prev_left.v, &x.v) {
                    prev_left.v.merge(x.v);
                } else {
                    let value = self.prev_left.replace(x).unwrap();
                    self.push_left(value);
                }
            }
            None => self.prev_left = Some(x),
        }
    }

    pub(crate) fn right(&mut self, x: KeyVal<K, V>) {
        match &mut self.prev_right {
            Some(prev_right) => {
                // merge into the previous right entry if the key is the same and the values are mergeable
                if prev_right.k == x.k && V::mergeable(&prev_right.v, &x.v) {
                    prev_right.v.merge(x.v);
                } else {
                    let value = self.prev_right.replace(x).unwrap();
                    self.push_right(value);
                }
            }
            None => self.prev_right = Some(x),
        }
    }
}

type Ranks<T> = (T, T);

/// Note that this cache fulfills a different purpose than merging because it can be used even when merging cannot.
/// For example, when you want to maintain distinct symbols but they share a boundary
pub(crate) struct RangedRankCache<V: BitVec> {
    end_index: Option<u32>, // previous end index
    end_ranks: Ranks<u32>,  // previous end ranks
    // note: we track these just out of interest;
    // we could enable only when profiling.
    // num_hits: usize,   // number of cache hits
    // num_misses: usize, // number of cache misses
    _v: PhantomData<V>,
}

impl<V: BitVec> RangedRankCache<V> {
    pub(crate) fn new() -> Self {
        Self {
            end_index: None,
            end_ranks: (0, 0),
            _v: PhantomData,
        }
    }

    pub(crate) fn get(
        &mut self,
        start_index: u32,
        end_index: u32,
        bv: &V,
    ) -> (Ranks<u32>, Ranks<u32>) {
        let start_ranks = if Some(start_index) == self.end_index {
            // self.num_hits += 1;
            self.end_ranks
        } else {
            // self.num_misses += 1;
            bv.ranks(start_index)
        };
        self.end_index = Some(end_index);
        self.end_ranks = bv.ranks(end_index);
        (start_ranks, self.end_ranks)
    }
}

// Mask stuff
//

// Return the union of set bits across all masks in `masks`
pub(crate) fn union_masks(masks: impl IntoIterator<Item = u32>) -> u32 {
    masks.into_iter().reduce(set_bits).unwrap_or(0)
}

// todo: accept a reference
pub(crate) fn mask_range(range: Range<u32>, mask: u32) -> Range<u32> {
    (range.start & mask)..((range.end - 1) & mask) + 1
}

pub(crate) fn mask_range_inclusive(extent: &RangeInclusive<u32>, mask: u32) -> RangeInclusive<u32> {
    extent.start() & mask..=extent.end() & mask
}

pub(crate) fn mask_symbol(symbol: u32, mask: u32) -> u32 {
    symbol & mask
}

pub(crate) fn masked(symbol: u32, mask: u32) -> u32 {
    symbol & mask
}

pub(crate) fn set_bits(value: u32, mask: u32) -> u32 {
    value | mask
}

pub(crate) fn unset_bits(value: u32, mask: u32) -> u32 {
    value & !mask
}

// given a current acc value, compute the acc value after visiting the node represented by `node_range`
// when the target search range is `symbol_range`.
// basically, decide whether to set or un-set the bits based on whether the node range is fully contained
// within symbol_range.
pub(crate) fn accumulate_mask(
    node_range: Range<u32>,
    mask: u32,
    symbol_range: &Range<u32>,
    accumulator: u32,
) -> u32 {
    toggle_bits(
        accumulator,
        mask,
        symbol_range.fully_contains_range(mask_range(node_range, mask)),
    )
}

// accumulator represents an accumulated mask consisting of the set/unset
// bits resulting from previous calls to this function.
// the idea is that we want to toggle individual masks on and off
// such that we can detect if there is ever a time that all have
// been turned on.
// since mask bits are disjoint (eg. the x bits are distinct from
// y bits in 2d morton order), we can tell whether they're all set
// by checking equality with u32::MAX.
// This function conditionally toggles the bits in `accumulator` specified by `mask`
// on or off, based on the value of `cond`.
pub(crate) fn toggle_bits(accumulator: u32, mask: u32, cond: bool) -> u32 {
    if cond {
        set_bits(accumulator, mask)
    } else {
        unset_bits(accumulator, mask)
    }
}

pub(crate) trait RangeOverlaps {
    /// Return true if `self` overlaps `other`
    fn overlaps(self, other: Self) -> bool;
    /// Return true if `self` fully contains `other`
    fn fully_contains(self, other: Self) -> bool;
    /// Return true if `self` overlaps the range `other`
    fn overlaps_range(self, other: Range<u32>) -> bool;
    /// Return true if `self` fully contains `other`
    fn fully_contains_range(self, other: Range<u32>) -> bool;
}

impl RangeOverlaps for &Range<u32> {
    fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    fn fully_contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    fn overlaps_range(self, other: Range<u32>) -> bool {
        self.overlaps(&other)
    }

    fn fully_contains_range(self, other: Range<u32>) -> bool {
        self.fully_contains(&other)
    }
}

impl RangeOverlaps for &RangeInclusive<u32> {
    fn overlaps(self, other: Self) -> bool {
        self.start() <= other.end() && other.start() <= self.end()
    }

    fn fully_contains(self, other: Self) -> bool {
        self.start() <= other.start() && self.end() >= other.end()
    }

    fn overlaps_range(self, other: Range<u32>) -> bool {
        *self.start() < other.end && other.start <= *self.end()
    }

    fn fully_contains_range(self, other: Range<u32>) -> bool {
        *self.start() <= other.start && other.end.saturating_sub(1) <= *self.end()
    }
}
