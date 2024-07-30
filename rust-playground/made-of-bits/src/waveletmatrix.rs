use crate::bitvec::BitVec;
use crate::bitvec::BitVecBuilder;
use crate::{
    bits::reverse_low_bits,
    bitvec::dense::{DenseBitVec, DenseBitVecBuilder},
};
use std::ops::Range;

#[derive(Debug)]
pub struct WaveletMatrix<V: BitVec> {
    levels: Vec<Level<V>>, // wm levels (bit planes)
    max_symbol: u32,       // maximum symbol value
    len: u32,              // number of symbols
}

#[derive(Debug)]
struct Level<V: BitVec> {
    bv: V,
    // the number of zeros at this level (ie. bv.rank0(bv.universe_size())
    nz: u32,
    // unsigned int with a single bit set signifying
    // the magnitude represented at that level.
    // e.g.  levels[0].bit == 1 << levels.len() - 1
    bit: u32,
}

impl<V: BitVec> Level<V> {
    // Returns (rank0(index), rank1(index))
    // This means that if x = ranks(index), x.0 is rank0 and x.1 is rank1.
    pub fn ranks(&self, index: u32) -> Ranks<u32> {
        if index == 0 {
            return Ranks(0, 0);
        }
        let num_ones = self.bv.rank1(index);
        let num_zeros = index - num_ones;
        Ranks(num_zeros, num_ones)
    }

    // Given the start index of a left node on this level, return the split points
    // that cover the range:
    // - left is the start of the left node
    // - mid is the start of the right node
    // - right is one past the end of the right node
    pub fn splits(&self, left: u32) -> (u32, u32, u32) {
        (left, left + self.bit, left + self.bit + self.bit)
    }
}

// Stores (rank0, rank1) as resulting from the Level::ranks function
#[derive(Copy, Clone)]
struct Ranks<T>(T, T);

impl<V: BitVec> WaveletMatrix<V> {
    pub fn new(data: Vec<u32>, max_symbol: u32) -> WaveletMatrix<DenseBitVec> {
        let num_levels = (u32::BITS - max_symbol.leading_zeros()).max(1);

        // We implement two different wavelet matrix construction algorithms. One of them is more
        // efficient, but that algorithm does not scale well to large alphabets and also cannot
        // cannot handle element multiplicity because it constructs the bitvectors out-of-order.
        // It also requires O(2^num_levels) space. So, we check whether the number of data points
        // is less than 2^num_levels, and if so use the scalable algorithm, and otherise use the
        // the efficient algorithm.
        let len = data.len();
        let levels = if len == 0 {
            vec![]
        } else if num_levels <= len.ilog2() {
            build_bitvecs(data, num_levels as usize)
        } else {
            build_bitvecs_large_alphabet(data, num_levels as usize)
        };

        WaveletMatrix::from_bitvecs(levels, max_symbol)
    }

    fn from_bitvecs(levels: Vec<V>, max_symbol: u32) -> WaveletMatrix<V> {
        let max_level = levels.len() - 1;
        let len = levels
            .first()
            .map(|level| level.universe_size())
            .unwrap_or(0);
        let levels: Vec<Level<V>> = levels
            .into_iter()
            .enumerate()
            .map(|(index, bits)| Level {
                nz: bits.rank0(bits.universe_size()),
                bit: 1 << (max_level - index),
                bv: bits,
            })
            .collect();
        let num_levels = levels.len();
        Self {
            levels,
            max_symbol,
            len,
        }
    }

    // Locate a symbol on the virtual bottom level of the wavelet tree.
    // Returns two things, both restricted to the query range:
    // - the number of symbols preceding this one in sorted order (less than)
    // - the range of this symbol on the virtual bottom level
    // This function is designed for internal use, where knowing the precise
    // range on the virtual level can be useful, e.g. for select queries.
    // Since the range also tells us the count of this symbol in the range, we
    // can combine the two pieces of data together for a count-less-than-or-equal query.
    // We compute both of these in one function since it's pretty cheap to do so.
    fn locate(&self, symbol: u32, range: Range<u32>, ignore_bits: usize) -> (u32, Range<u32>) {
        assert!(
            symbol <= self.max_symbol,
            "symbol must not exceed max_symbol"
        );
        let mut preceding_count = 0;
        let mut range = range;
        for level in self.levels(ignore_bits) {
            let start = level.ranks(range.start);
            let end = level.ranks(range.end);
            // check if the symbol's level bit is set to determine whether it should be mapped
            // to the left or right child node
            if symbol & level.bit == 0 {
                // go left
                range = start.0..end.0;
            } else {
                // count the symbols in the left child before going right
                preceding_count += end.0 - start.0;
                range = level.nz + start.1..level.nz + end.1;
            }
        }
        (preceding_count, range)
    }

    /// Number of symbols less than this one, restricted to the query range
    pub fn preceding_count(&self, symbol: u32, range: Range<u32>) -> u32 {
        self.locate(symbol, range, 0).0
    }

    /// Number of times the symbol appears in the query range
    pub fn count(&self, range: Range<u32>, symbol: u32) -> u32 {
        let range = self.locate(symbol, range, 0).1;
        range.end - range.start
    }

    /// Returns (symbol, count)
    pub fn quantile(&self, k: u32, range: Range<u32>) -> (u32, u32) {
        assert!(k < range.end - range.start);
        let mut k = k;
        let mut range = range;
        let mut symbol = 0;
        for level in self.levels(0) {
            let start = level.ranks(range.start);
            let end = level.ranks(range.end);
            let left_count = end.0 - start.0;
            if k < left_count {
                // go left
                range = start.0..end.0;
            } else {
                // go right
                k -= left_count;
                symbol += level.bit;
                range = level.nz + start.1..level.nz + end.1;
            }
        }
        let count = range.end - range.start;
        (symbol, count)
    }

    /// Return the index of the k-th occurrence of `symbol`
    pub fn select(
        &self,
        symbol: u32,
        k: u32,
        range: Range<u32>,
        ignore_bits: usize,
    ) -> Option<u32> {
        if symbol > self.max_symbol {
            return None;
        }

        // track the symbol down to a range on the bottom-most level we're interested in
        let range = self.locate(symbol, range, ignore_bits).1;
        let count = range.end - range.start;

        // If there are fewer than `k+1` copies of `symbol` in the range, return early.
        // `k` is zero-indexed, so our check includes equality.
        if count <= k {
            return None;
        }

        // track the k-th occurrence of the symbol up from the bottom-most virtual level
        // or higher, if ignore_bits is non-zero.
        let index = range.start + k;
        self.select_upwards(index, ignore_bits)
    }

    // Return the index of the k-th occurrence of `symbol` from the back of the wavelet matrix
    pub fn select_last(
        &self,
        symbol: u32,
        k: u32,
        range: Range<u32>,
        ignore_bits: usize,
    ) -> Option<u32> {
        if symbol > self.max_symbol {
            return None;
        }
        let range = self.locate(symbol, range, ignore_bits).1;
        let count = range.end - range.start;
        if count <= k {
            return None;
        }
        let index = range.end - k - 1; // - 1 because end is exclusive
        self.select_upwards(index, ignore_bits)
    }

    // This function abstracts the common second half of the select algorithm, once you've
    // identified an index on the "bottom" level and want to bubble it back up to translate
    // the "sorted" index from the bottom level to the index of that element in sequence order.
    // We make this a pub fn since it could allow eg. external users of `locate` to efficiently
    // select their chosen element. For example, perhaps we should remove `select_last`...
    pub fn select_upwards(&self, index: u32, ignore_bits: usize) -> Option<u32> {
        let mut index = index;
        for level in self.levels(ignore_bits).rev() {
            // `index` represents an index on the level below this one, which may be
            // the bottom-most 'virtual' layer that contains all symbols in sorted order.
            //
            // We want to determine the position of the element represented by `index` on
            // this level, which we can do by "mapping" the index up to its parent node.
            //
            // `level.nz` tells us how many bits on the level below come from left children of
            // the wavelet tree (represented by this wavelet matrix). If the index < nz, that
            // means that the index on the level below came from a left child on this level,
            // which means that it must be represented by a 0-bit on this level; specifically,
            // the `index`-th 0-bit, since the WT always represents a stable sort of its elements.
            //
            // On the other hand, if `index` came from a right child on this level, then it
            // is represented by a 1-bit on this level; specifically, the `index - nz`-th 1-bit.
            //
            // In either case, we can use bitvector select to compute the index on this level.
            if index < level.nz {
                // `index` represents a left child on this level, represented by the `index`-th 0-bit.
                index = level.bv.select0(index).unwrap();
            } else {
                // `index` represents a right child on this level, represented by the `index-nz`-th 1-bit.
                index = level.bv.select1(index - level.nz).unwrap();
            }
        }
        Some(index)
    }

    pub fn get(&self, index: u32) -> u32 {
        let mut index = index;
        let mut symbol = 0;
        for level in self.levels(0) {
            if level.bv.get(index) == 0 {
                // go left
                index = level.bv.rank0(index);
            } else {
                // go right
                symbol += level.bit;
                index = level.nz + level.bv.rank1(index);
            }
        }
        symbol
    }

    /// Return the majority element, if one exists.
    /// The majority element is one whose frequency is larger than 50% of the range.
    pub fn simple_majority(&self, range: Range<u32>) -> Option<u32> {
        let len = range.end - range.start;
        let half_len = len >> 1;
        let (symbol, count) = self.quantile(half_len, range);
        if count > half_len {
            Some(symbol)
        } else {
            None
        }
    }

    // todo: fn k_majority(&self, k, range) { ... }

    // Returns an iterator over levels from the high bit downwards, ignoring the
    // bottom `ignore_bits` levels.
    fn levels(&self, ignore_bits: usize) -> std::slice::Iter<Level<V>> {
        self.levels[..self.levels.len() - ignore_bits].iter()
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn max_symbol(&self) -> u32 {
        self.max_symbol
    }

    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }
}

// Wavelet matrix construction algorithm optimized for the case where we can afford to build a
// dense histogram that counts the number of occurrences of each symbol. Heuristically,
// this is roughly the case where the alphabet size does not exceed the number of data points.
// Implements Algorithm 1 (seq.pc) from the paper "Practical Wavelet Tree Construction".
fn build_bitvecs(data: Vec<u32>, num_levels: usize) -> Vec<DenseBitVec> {
    assert!(data.len() <= u32::MAX as usize);
    let mut levels = vec![DenseBitVecBuilder::new(data.len() as u32); num_levels];
    let mut hist = vec![0; 1 << num_levels];
    let mut borders = vec![0; 1 << num_levels];
    let max_level = num_levels - 1;

    {
        // Count symbol occurrences and fill the first bitvector, whose bits
        // can be read from MSBs of the data in its original order.
        let level = &mut levels[0];
        let level_bit = 1 << max_level;
        for (i, &d) in data.iter().enumerate() {
            hist[d as usize] += 1;
            if d & level_bit > 0 {
                level.one(i as u32);
            }
        }
    }

    // Construct the other levels bottom-up
    for l in (1..num_levels).rev() {
        // The number of wavelet tree nodes at this level
        let num_nodes = 1 << l;

        // Compute the histogram based on the previous level's histogram
        for i in 0..num_nodes {
            // Update the histogram in-place
            hist[i] = hist[2 * i] + hist[2 * i + 1];
        }

        // Get starting positions of intervals from the new histogram
        borders[0] = 0;
        for i in 1..num_nodes {
            // Update the positions in-place. The bit reversals map from wavelet tree
            // node order to wavelet matrix node order, with all left children preceding
            // the right children.
            let prev_index = reverse_low_bits(i - 1, l);
            borders[reverse_low_bits(i, l)] = borders[prev_index] + hist[prev_index];
        }

        // Fill the bit vector of the current level
        let level = &mut levels[l];
        let level_bit_index = max_level - l;
        let level_bit = 1 << level_bit_index;

        // This mask contains all ones except for the lowest level_bit_index bits.
        let bit_prefix_mask = usize::MAX
            .checked_shl((level_bit_index + 1) as u32)
            .unwrap_or(0);
        for &d in data.iter() {
            // Get and update position for bit by computing its bit prefix from the
            // MSB downwards which encodes the path from the root to the node at
            // this level that contains this bit
            let node_index = (d as usize & bit_prefix_mask) >> (level_bit_index + 1);
            let p = &mut borders[node_index];
            // Set the bit in the bitvector
            if d & level_bit > 0 {
                level.one(*p);
            }
            *p += 1;
        }
    }

    levels.into_iter().map(|level| level.build()).collect()
}

/// Wavelet matrix construction algorithm optimized for large alphabets.
/// Returns an array of level bitvectors built from `data`.
/// Handles the sparse case where the alphabet size exceeds the number of data points and
/// building a histogram with an entry for each symbol is expensive.
fn build_bitvecs_large_alphabet(mut data: Vec<u32>, num_levels: usize) -> Vec<DenseBitVec> {
    assert!(data.len() <= u32::MAX as usize);
    let mut levels = Vec::with_capacity(num_levels);
    let max_level = num_levels - 1;

    // For each level, stably sort the datapoints by their bit value at that level.
    // Elements with a zero bit get sorted left, and elements with a one bits
    // get sorted right, which is effectvely a bucket sort with two buckets.
    let mut right = Vec::new();

    for l in 0..max_level {
        let level_bit = 1 << (max_level - l);
        let mut b = DenseBitVecBuilder::new(data.len() as u32);
        let mut index = 0;
        // Stably sort all elements with a zero bit at this level to the left, storing
        // the positions of all one bits at this level in `bits`.
        // We retain the elements that went left, then append those that went right.
        data.retain_mut(|d| {
            let value = *d;
            let go_left = value & level_bit == 0;
            if !go_left {
                b.one(index);
                right.push(value);
            }
            index += 1;
            go_left
        });
        data.append(&mut right);
        levels.push(b.build());
    }

    // For the last level we don't need to do anything but build the bitvector
    {
        let mut b = DenseBitVecBuilder::new(data.len() as u32);
        let level_bit = 1 << 0;
        for (index, d) in data.iter().enumerate() {
            if d & level_bit > 0 {
                b.one(index as u32);
            }
        }
        levels.push(b.build());
    }

    levels
}

// Return true if `a` overlaps `b`
fn range_overlaps(a: &Range<u32>, b: &Range<u32>) -> bool {
    a.start < b.end && b.start < a.end
}

// Return true if `a` fully contains `b`
fn range_fully_contains(a: &Range<u32>, b: &Range<u32>) -> bool {
    // if a starts before b, and a ends after b.
    a.start <= b.start && a.end >= b.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panics;

    #[test]
    fn spot_test() {
        //    values:   1  3  3  2  7
        //              -------------
        //    bits
        //    level 0:  0  0  0  0  1  bit 2^2 = 4
        //    level 1:  0  1  1  1  1  bit 2^1 = 2
        //    level 2:  1  1  1  0  1  bit 2^0 = 1
        let data = vec![1, 3, 3, 2, 7];
        let len = data.len() as u32;
        let max_symbol = data.iter().max().copied().unwrap();
        let wm = WaveletMatrix::<DenseBitVec>::new(data.clone(), max_symbol);

        {
            // num_levels
            assert_eq!(wm.num_levels(), 3);
        }

        {
            // get
            assert_eq!(
                data.iter()
                    .enumerate()
                    .map(|(i, _)| wm.get(i as u32))
                    .collect::<Vec<_>>(),
                data
            );

            // get: out of bounds
            assert!(panics(|| wm.get(6)));
        }

        {
            // select
            assert_eq!(wm.select(0, 0, 0..len, 0), None);
            assert_eq!(wm.select(1, 0, 0..len, 0), Some(0));
            assert_eq!(wm.select(2, 0, 0..len, 0), Some(3));
            assert_eq!(wm.select(3, 0, 0..len, 0), Some(1));
            assert_eq!(wm.select(3, 1, 0..len, 0), Some(2));
            assert_eq!(wm.select(7, 0, 0..len, 0), Some(4));
            assert_eq!(wm.select(5, 0, 0..len, 0), None);

            // select with 2 ignore_bits
            assert_eq!(wm.select(0, 0, 0..len, 2), Some(0));
            assert_eq!(wm.select(1, 0, 0..len, 2), Some(0));
            assert_eq!(wm.select(2, 0, 0..len, 2), Some(0));
            assert_eq!(wm.select(3, 0, 0..len, 2), Some(0));
            assert_eq!(wm.select(3, 1, 0..len, 2), Some(1));
            assert_eq!(wm.select(7, 0, 0..len, 2), Some(4));
            assert_eq!(wm.select(5, 0, 0..len, 2), Some(4));
            assert_eq!(wm.select(100, 0, 0..len, 2), None);

            // select with full ignore_bits
            assert_eq!(wm.select(0, 0, 0..len, wm.num_levels()), Some(0));
            assert_eq!(wm.select(1, 0, 0..len, wm.num_levels()), Some(0));
            assert_eq!(wm.select(2, 0, 0..len, wm.num_levels()), Some(0));
            assert_eq!(wm.select(3, 0, 0..len, wm.num_levels()), Some(0));
            assert_eq!(wm.select(3, 1, 0..len, wm.num_levels()), Some(1));
            assert_eq!(wm.select(7, 0, 0..len, wm.num_levels()), Some(0));
            assert_eq!(wm.select(5, 0, 0..len, wm.num_levels()), Some(0));
            assert_eq!(wm.select(100, 0, 0..len, wm.num_levels()), None);
        }

        {
            // select_last
            assert_eq!(wm.select_last(0, 0, 0..len, 0), None);
            assert_eq!(wm.select_last(1, 0, 0..len, 0), Some(0));
            assert_eq!(wm.select_last(2, 0, 0..len, 0), Some(3));
            assert_eq!(wm.select_last(3, 0, 0..len, 0), Some(2));
            assert_eq!(wm.select_last(3, 1, 0..len, 0), Some(1));
            assert_eq!(wm.select_last(7, 0, 0..len, 0), Some(4));
            assert_eq!(wm.select_last(5, 0, 0..len, 0), None);

            // select_last with 2 ignore_bits (just 1 not-ignored bit)
            assert_eq!(wm.select_last(0, 0, 0..len, 2), Some(3));
            assert_eq!(wm.select_last(1, 0, 0..len, 2), Some(3));
            assert_eq!(wm.select_last(2, 0, 0..len, 2), Some(3));
            assert_eq!(wm.select_last(3, 0, 0..len, 2), Some(3));
            assert_eq!(wm.select_last(3, 1, 0..len, 2), Some(2));
            assert_eq!(wm.select_last(7, 0, 0..len, 2), Some(4));
            assert_eq!(wm.select_last(5, 0, 0..len, 2), Some(4));
            assert_eq!(wm.select_last(100, 0, 0..len, 2), None);

            // select_last with full ignore_bits
            assert_eq!(wm.select_last(0, 0, 0..len, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(1, 0, 0..len, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(2, 0, 0..len, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(3, 0, 0..len, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(3, 1, 0..len, wm.num_levels()), Some(3));
            assert_eq!(wm.select_last(7, 0, 0..len, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(5, 0, 0..len, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(100, 0, 0..len, wm.num_levels()), None);
        }

        {
            // simple_majority
            assert_eq!(wm.simple_majority(0..len), None);
            assert_eq!(wm.simple_majority(0..3), Some(3));
            assert_eq!(wm.simple_majority(0..1), Some(1));
            assert_eq!(wm.simple_majority(1..len - 1), Some(3));
            assert_eq!(wm.simple_majority(1..len), None);
        }

        {
            // quantile
            assert_eq!(wm.quantile(0, 0..len), (1, 1));
            assert_eq!(wm.quantile(1, 0..len), (2, 1));
            assert_eq!(wm.quantile(2, 0..len), (3, 2));
            assert_eq!(wm.quantile(3, 0..len), (3, 2));
            assert_eq!(wm.quantile(4, 0..len), (7, 1));

            // multiplicity is within the reduced range
            assert_eq!(wm.quantile(0, 1..2), (3, 1));

            // quantile: check all values within a tighter range
            assert_eq!(wm.quantile(0, 1..len - 1), (2, 1));
            assert_eq!(wm.quantile(1, 1..len - 1), (3, 2));
            assert_eq!(wm.quantile(2, 1..len - 1), (3, 2));

            // quantile: out of bounds
            assert!(panics(|| wm.quantile(3, 1..len - 1)));
        }

        {
            // preceding_count
            assert_eq!(wm.preceding_count(0, 0..len), 0);
            assert_eq!(wm.preceding_count(1, 0..len), 0);
            assert_eq!(wm.preceding_count(2, 0..len), 1);
            assert_eq!(wm.preceding_count(3, 0..len), 2);
            assert_eq!(wm.preceding_count(4, 0..len), 4);
            assert_eq!(wm.preceding_count(5, 0..len), 4);
            assert_eq!(wm.preceding_count(7, 0..len), 4);

            // preceding_count: symbol is beyond max_symbol
            assert!(panics(|| wm.preceding_count(max_symbol + 1, 0..len)));
        }
    }
}
