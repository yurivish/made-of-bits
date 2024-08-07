use std::ops::{Range, RangeInclusive};
// The traversal order means that outputs do not appear in the same order as inputs and
// there may be multiple outputs per input (e.g. symbols found within a given index range)
// so associating each batch with an index allows us to track the association between inputs
// and outputs.
// The Key is (currently) the input index associated with this query, so we can track it through
// the tree.
use crate::bitvec::BitVec;
use core::marker::PhantomData;
use std::collections::VecDeque;

#[derive(Debug)]
pub(crate) struct Level<V: BitVec> {
    pub(crate) bv: V,
    // the number of zeros at this level (ie. bv.rank0(bv.universe_size())
    pub(crate) nz: u32,
    // unsigned int with a single bit set signifying
    // the magnitude represented at that level.
    // e.g.  levels[0].bit == 1 << levels.len() - 1
    pub(crate) bit: u32,
}

impl<V: BitVec> Level<V> {
    // Returns (rank0(index), rank1(index))
    // This means that if x = ranks(index), x.0 is rank0 and x.1 is rank1.
    pub(crate) fn ranks(&self, index: u32) -> Ranks<u32> {
        let (num_zeros, num_ones) = self.bv.ranks(index);
        Ranks(num_zeros, num_ones)
    }

    // Given the start index of a left node on this level, return the split points
    // that cover the range:
    // - left is the start of the left node
    // - mid is the start of the right node
    // - right is one past the end of the right node
    pub(crate) fn splits(&self, left: u32) -> (u32, u32, u32) {
        (left, left + self.bit, left + self.bit + self.bit)
    }

    pub(crate) fn child_symbol_extents(
        &self,
        left: u32,
        mask: u32,
    ) -> (RangeInclusive<u32>, RangeInclusive<u32>) {
        let (left, mid, right) = self.splits(left);
        (
            mask_extent(&(left..=mid - 1), mask),
            mask_extent(&(mid..=right - 1), mask),
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct KeyVal<T> {
    pub(crate) key: usize,
    pub(crate) val: T,
}

// Associate a usize key to an arbitrary value; used for propagating the metadata
// of which original query element a partial query result is associated with as we
// traverse the wavelet tree
impl<T> KeyVal<T> {
    pub(crate) fn new(key: usize, value: T) -> KeyVal<T> {
        KeyVal { key, val: value }
    }
    // construct a BatchValue from an (key, value) tuple
    pub(crate) fn from_tuple((key, value): (usize, T)) -> KeyVal<T> {
        KeyVal { key, val: value }
    }
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> KeyVal<U> {
        KeyVal {
            key: self.key,
            val: f(self.val),
        }
    }
    // return a new KeyVal with the previous key and new value
    pub(crate) fn val(self, value: T) -> KeyVal<T> {
        KeyVal { val: value, ..self }
    }
}

// Helper for traversing the wavelet matrix level by level,
// reusing space when possible and keeping the elements in
// sorted order with respect to the ordering of wavelet tree
// nodes in the wavelet matrix (all left nodes precede all
// right nodes).
#[derive(Debug)]
pub(crate) struct Traversal<T> {
    cur: VecDeque<KeyVal<T>>,
    next: VecDeque<KeyVal<T>>,
    num_left: usize,
}

// Traverse a wavelet matrix levelwise, at each level maintaining tree nodes
// in order they appear in the wavelet matrix (left children preceding right).
impl<T> Traversal<T> {
    pub(crate) fn new(values: impl IntoIterator<Item = T>) -> Self {
        let mut traversal = Self {
            cur: VecDeque::new(),
            next: VecDeque::new(),
            num_left: 0,
        };
        traversal.init(values);
        traversal
    }

    pub(crate) fn init(&mut self, values: impl IntoIterator<Item = T>) {
        let iter = values.into_iter().enumerate().map(KeyVal::from_tuple);
        self.cur.clear();
        self.next.clear();
        self.next.extend(iter);
        self.num_left = 0;
    }

    pub(crate) fn traverse(&mut self, mut f: impl FnMut(&[KeyVal<T>], &mut Goer<KeyVal<T>>)) {
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
        };

        // invoke the traversal function with the current elements and the recursion helper
        // we pass an iterator rather than an element at a time so that f can do its own
        // batching if it wants to
        f(cur, &mut go);

        // update the number of nodes that went left based on the calls `f` made to `go`
        self.num_left = go.num_left;
    }

    pub(crate) fn results(&mut self) -> &mut [KeyVal<T>] {
        let slice = self.next.make_contiguous();
        // note: reverse only required if we want to return results in wm order,
        // which might be nice if we are eg. looking up associated data.
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

// Passed into the traversal callback as a way to control the recursion.
// Goes left and/or right.
pub(crate) struct Goer<'a, T> {
    next: &'a mut VecDeque<T>,
    num_left: usize,
}

impl<T> Goer<'_, T> {
    pub(crate) fn left(&mut self, kv: T) {
        // left children are appended to the front of the queue
        // which causes them to be in reverse order
        self.next.push_front(kv);
        self.num_left += 1;
    }
    pub(crate) fn right(&mut self, kv: T) {
        // right children are appended to the back of the queue
        self.next.push_back(kv);
    }
}

pub(crate) struct RangedRankCache<V: BitVec> {
    end_index: Option<u32>, // previous end index
    end_ranks: Ranks<u32>,  // previous end ranks
    // note: we track these just out of interest;
    // we could enable only when profiling.
    num_hits: usize,   // number of cache hits
    num_misses: usize, // number of cache misses
    _v: PhantomData<V>,
}

impl<V: BitVec> RangedRankCache<V> {
    pub(crate) fn new() -> Self {
        Self {
            end_index: None,
            end_ranks: Ranks(0, 0),
            num_hits: 0,
            num_misses: 0,
            _v: PhantomData,
        }
    }

    pub(crate) fn get(
        &mut self,
        start_index: u32,
        end_index: u32,
        level: &Level<V>,
    ) -> (Ranks<u32>, Ranks<u32>) {
        let start_ranks = if Some(start_index) == self.end_index {
            self.num_hits += 1;
            self.end_ranks
        } else {
            self.num_misses += 1;
            level.ranks(start_index)
        };
        self.end_index = Some(end_index);
        self.end_ranks = level.ranks(end_index);
        (start_ranks, self.end_ranks)
    }

    pub(crate) fn log_stats(&self) {
        println!(
            "cached {:.1}%: {:?} / {:?}",
            // note: can be nan
            100.0 * self.num_hits as f64 / (self.num_hits + self.num_misses) as f64,
            self.num_hits,
            self.num_hits + self.num_misses,
        );
    }
}

// Stores (rank0, rank1) as resulting from the Level::ranks function
#[derive(Copy, Clone, Debug)]
pub(crate) struct Ranks<T>(pub(crate) T, pub(crate) T);

// Mask stuff

// Return the union of set bits across all masks in `masks`
pub(crate) fn union_masks(masks: &[u32]) -> u32 {
    masks.iter().copied().reduce(set_bits).unwrap_or(0)
}

pub(crate) fn mask_range(range: Range<u32>, mask: u32) -> Range<u32> {
    (range.start & mask)..((range.end - 1) & mask) + 1
}

pub(crate) fn mask_extent(extent: &RangeInclusive<u32>, mask: u32) -> RangeInclusive<u32> {
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
        symbol_range.contains_range(mask_range(node_range, mask)),
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
    fn overlaps_range(self, other: Self) -> bool;
    fn contains_range(self, other: Range<u32>) -> bool;
}

impl RangeOverlaps for &Range<u32> {
    /// Return true if `self` overlaps `other`
    fn overlaps_range(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Return true if `self` fully contains `other`
    fn contains_range(self, other: Range<u32>) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}

impl RangeOverlaps for &RangeInclusive<u32> {
    /// Return true if `self` overlaps `other`
    fn overlaps_range(self, other: Self) -> bool {
        self.start() <= other.end() && other.start() <= self.end()
    }

    /// Return true if `self` fully contains `other`
    fn contains_range(self, other: Range<u32>) -> bool {
        *self.start() <= other.start && *self.end() > other.end
    }
}
