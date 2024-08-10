use crate::bitvec::BitVec;
use crate::bitvec::BitVecBuilder;
use crate::waveletmatrix_support::accumulate_mask;
use crate::waveletmatrix_support::mask_extent;
use crate::waveletmatrix_support::mask_range;
use crate::waveletmatrix_support::union_masks;
use crate::waveletmatrix_support::RangeOverlaps;
use crate::waveletmatrix_support::Ranks;
use crate::waveletmatrix_support::{KeyVal, Level, RangedRankCache, Traversal};
use crate::DenseBitVecOptions;
use crate::{
    bits::reverse_low_bits,
    bitvec::dense::{DenseBitVec, DenseBitVecBuilder},
};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::Range;
use std::ops::RangeInclusive;

// todo: explore whether we can use a default value for V = DenseBitVec

#[derive(Debug)]
pub struct WaveletMatrix<V: BitVec> {
    levels: Vec<Level<V>>, // wm levels (bit planes)
    max_symbol: u32,       // maximum symbol value
    len: u32,              // number of symbols
    default_masks: Box<[u32]>,
}

impl<V: BitVec> WaveletMatrix<V> {
    pub fn new(
        data: Vec<u32>,
        max_symbol: u32,
        bitvec_options: Option<<V::Builder as BitVecBuilder>::Options>,
    ) -> WaveletMatrix<V> {
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
            Self::build_bitvecs(data, num_levels as usize, bitvec_options)
        } else {
            Self::build_bitvecs_large_alphabet(data, num_levels as usize, bitvec_options)
        };
        Self::from_bitvecs(levels, max_symbol)
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
            default_masks: vec![u32::MAX; num_levels].into(),
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
    pub fn locate(&self, range: Range<u32>, symbol: u32, ignore_bits: usize) -> (u32, Range<u32>) {
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
    pub fn preceding_count(&self, range: Range<u32>, symbol: u32) -> u32 {
        self.locate(range, symbol, 0).0
    }

    /// Number of times the symbol appears in the query range
    pub fn count(&self, range: Range<u32>, symbol: u32) -> u32 {
        let range = self.locate(range, symbol, 0).1;
        range.end - range.start
    }

    /// Returns (symbol, count)
    pub fn quantile(&self, range: Range<u32>, k: u32) -> (u32, u32) {
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
        range: Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32> {
        if symbol > self.max_symbol {
            return None;
        }

        // track the symbol down to a range on the bottom-most level we're interested in
        let range = self.locate(range, symbol, ignore_bits).1;
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
        range: Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32> {
        if symbol > self.max_symbol {
            return None;
        }
        let range = self.locate(range, symbol, ignore_bits).1;
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
        let (symbol, count) = self.quantile(range, half_len);
        if count > half_len {
            Some(symbol)
        } else {
            None
        }
    }

    // todo: fn k_majority(&self, k, range) { ... }

    // Count the number of occurrences of each symbol in the given index ranges.
    // Returns a vec of (input_index, symbol, start, end) tuples.
    // Returning (start, end) rather than a count `end - start` is helpful for
    // use cases that associate per-symbol data with each entry.
    // Note that (when ignore_bits is 0) the range is on the virtual bottom layer
    // of the wavelet matrix, where symbols are sorted in ascending bit-reversed order.
    // TODO: Is there a way to do ~half the number of rank queries for contiguous
    // ranges that share a midpoint, ie. [a..b, b..c, c..d]?
    pub fn counts(
        &self,
        ranges: &[Range<u32>],
        // note: this is inclusive because the requirement comes up in practice, eg.
        // a 32-bit integer can represent 3 10-bit integers, and you may want to query
        // for the 10-bit subcomponents separately, eg. 0..1025. But to represent 1025
        // in each dimension would require 33 bits. instead, inclusive extents allow
        // representing 0..1025 (11 bits) as 0..=1024 (10 bits).
        symbol_extent: RangeInclusive<u32>,
        masks: Option<&[u32]>,
    ) -> Traversal<Counts> {
        let masks = masks.unwrap_or(&self.default_masks);

        for range in ranges {
            assert!(range.end <= self.len());
        }

        let mut traversal = Traversal::new(ranges.iter().map(|range| Counts {
            symbol: 0, // the leftmost symbol in the current node
            start: range.start,
            end: range.end,
        }));

        for (level, mask) in self.levels.iter().zip(masks.iter().copied()) {
            let symbol_extent = mask_extent(&symbol_extent, mask);
            traversal.traverse(|xs, go| {
                // Cache the most recent rank call in case the next one is the same.
                // This means caching the `end` of the previous range, and checking
                // if it is the same as the `start` of the current range.
                let mut rank_cache = RangedRankCache::new();
                for x in xs {
                    let symbol = x.val.symbol;
                    let (left, right) = level.child_symbol_extents(symbol, mask);
                    let (start, end) = rank_cache.get(x.val.start, x.val.end, level);
                    // if there are any left children, go left
                    if start.0 != end.0 && symbol_extent.overlaps_range(&left) {
                        go.left(x.val(Counts::new(symbol, start.0, end.0)));
                    }

                    // if there are any right children, set the level bit and go right
                    if start.1 != end.1 && symbol_extent.overlaps_range(&right) {
                        go.right(x.val(Counts::new(
                            symbol + level.bit,
                            level.nz + start.1,
                            level.nz + end.1,
                        )));
                    }
                }
                // rank_cache.log_stats();
            });
        }
        traversal
    }

    pub fn counts_faster_maybe(&self, ranges: &[Range<u32>]) -> Traversal<Counts> {
        for range in ranges {
            assert!(range.end <= self.len());
        }

        let mut traversal = Traversal::new(ranges.iter().map(|range| Counts {
            symbol: 0, // the leftmost symbol in the current node
            start: range.start,
            end: range.end,
        }));

        let mut bit_indices = vec![];
        let mut batch_ranks = vec![];

        for level in self.levels.iter() {
            bit_indices.clear();

            // merge traversal.
            // also accumulate bit indices for rank queries, so we can do them in a batch.
            traversal.traverse(|xs, go| {
                // println!("pre-merge: {}", xs.len());
                if let Some(first) = xs.first() {
                    let mut prev: KeyVal<Counts> = *first;
                    for x in &xs[1..] {
                        let cur = x;
                        if prev.val.symbol == cur.val.symbol && prev.val.end == cur.val.start {
                            prev.val.end = cur.val.end;
                        } else {
                            debug_assert!(prev.val.start <= prev.val.end);
                            bit_indices.push(prev.val.start);
                            bit_indices.push(prev.val.end);
                            go.right(prev);
                            prev = *cur;
                        }
                    }
                    debug_assert!(prev.val.start <= prev.val.end);
                    bit_indices.push(prev.val.start);
                    bit_indices.push(prev.val.end);
                    go.right(prev);
                }
            });

            batch_ranks.clear();
            level.bv.rank1_batch(&mut batch_ranks, &bit_indices);

            traversal.traverse(|xs, go| {
                for (x, r) in xs.iter().zip(batch_ranks.chunks_exact(2)) {
                    let (start, end) = {
                        let start1 = r[0];
                        let end1 = r[1];
                        let start0 = x.val.start - start1;
                        let end0 = x.val.end - end1;
                        ((start0, start1), (end0, end1))
                    };
                    // if there are any left children, go right
                    if start.0 != end.0 {
                        go.left(x.val(Counts::new(x.val.symbol, start.0, end.0)));
                    }
                    // if there are any right children, set the level bit and go right
                    if start.1 != end.1 {
                        go.right(x.val(Counts::new(
                            x.val.symbol + level.bit,
                            level.nz + start.1,
                            level.nz + end.1,
                        )));
                    }
                }
            });
        }

        traversal
    }

    pub fn default_masks(&self) -> &Box<[u32]> {
        &self.default_masks
    }

    pub fn morton_masks_for_dims(&self, dims: u32) -> Vec<u32> {
        const S1: [u32; 1] = [u32::MAX];
        const S2: [u32; 2] = [
            crate::zorder::encode2(0, u32::MAX),
            crate::zorder::encode2(u32::MAX, 0),
        ];
        const S3: [u32; 3] = [
            crate::zorder::encode3(0, 0, u32::MAX),
            crate::zorder::encode3(0, u32::MAX, 0),
            crate::zorder::encode3(u32::MAX, 0, 0),
        ];
        let masks = match dims {
            1 => &S1[..],
            2 => &S2[..],
            3 => &S3[..],
            _ => panic!("only 1-3 dimensions currently supported"),
        };
        masks
            .iter()
            .copied()
            .cycle()
            .take(self.num_levels())
            .collect()
    }

    // Count the number of occurences of symbols in each of the symbol ranges,
    // returning a parallel array of counts.
    // Range is an index range.
    // Masks is a slice of bitmasks, one per level, indicating the bitmask operational
    // at that level, to enable multidimensional queries.
    // To search in 1d, pass std::iter::repeat(u32::MAX).take(wm.num_levels()).collect().
    pub fn count_batch(
        &self,
        range: Range<u32>,
        symbol_ranges: &[Range<u32>],
        masks: Option<&[u32]>,
    ) -> Vec<u32> {
        let masks = masks.unwrap_or(&self.default_masks);

        // Union all bitmasks so we can tell when we the symbol range is fully contained within
        // the query range at a particular wavelet tree node, in order to avoid needless recursion.
        let all_masks = union_masks(masks);

        // The return vector of counts
        let mut counts = vec![0; symbol_ranges.len()];

        // Initialize a wavelet matrix traversal with one entry per symbol range we're searching.
        let init = CountSymbolRange::new(0, 0, range.start, range.end);
        let mut traversal = Traversal::new(std::iter::repeat(init).take(symbol_ranges.len()));

        for (level, mask) in self.levels.iter().zip(masks.iter().copied()) {
            traversal.traverse(|xs, go| {
                // Cache rank queries when the start of the current range is the same as the end of the previous range
                let mut rank_cache = RangedRankCache::new();
                for x in xs {
                    // The symbol range corresponding to the current query, masked to the relevant dimensions at this level
                    let symbol_range = mask_range(symbol_ranges[x.key].clone(), mask);

                    // Left, middle, and right symbol indices for the children of this node.
                    let (left, mid, right) = level.splits(x.val.left);

                    // Tuples representing the rank0/1 of start and rank0/1 of end.
                    let (start, end) = rank_cache.get(x.val.start, x.val.end, level);

                    // Check the left child if there are any elements there
                    if start.0 != end.0 {
                        // Determine whether we can short-circuit the recursion because the symbols
                        // represented by the left child are fully contained in symbol_range in all
                        // dimensions (ie. for all distinct masks). For example, if the masks represent
                        // a two-dimensional query, we need to check that (effectively) the quadtree
                        // node, represented by two contiguous dimensions, is contained. It's a bit subtle
                        // since we can early-out not only if a contiguous 'xy' range is detected, but also
                        // a contiguous 'yx' range – so long as the symbol range is contained in the most
                        // recent branching in all dimensions, we can stop the recursion early and count the
                        // node's children, since that means all children are contained within the query range.
                        //
                        // Each "dimension" is indicated by a different mask. So far, use cases have meant that
                        // each bit of the symbol is assigned to at most one mask.
                        //
                        // To accumulate a new mask to the accumulator, we will either set or un-set all the bits
                        // corresponding to this mask. We will set them if the symbol range represented by this node
                        // is fully contained in the query range, and un-set them otherwise.
                        //
                        // If the node is contained in all dimensions, then the accumulator will be equal to all_masks,
                        // and we can stop the recursion early.
                        let acc = accumulate_mask(left..mid, mask, &symbol_range, x.val.acc);
                        if acc == all_masks {
                            counts[x.key] += end.0 - start.0;
                        } else if symbol_range.overlaps_range(&mask_range(left..mid, mask)) {
                            // We need to recurse into the left child. Do so with the new acc value.
                            go.left(x.val(CountSymbolRange::new(acc, left, start.0, end.0)));
                        }
                    }

                    // right child
                    if start.1 != end.1 {
                        // See the comments for the left node; the logical structure here is identical.
                        let acc = accumulate_mask(mid..right, mask, &symbol_range, x.val.acc);
                        if acc == all_masks {
                            counts[x.key] += end.1 - start.1;
                        } else if symbol_range.overlaps_range(&mask_range(mid..right, mask)) {
                            go.right(x.val(CountSymbolRange::new(
                                acc,
                                mid,
                                level.nz + start.1,
                                level.nz + end.1,
                            )));
                        }
                    }
                }
            });
        }

        // For complete queries, the last iteration of the loop above finds itself recursing to the
        // virtual bottom level of the wavelet tree, each node representing an individual symbol,
        // so there should be no uncounted nodes left over. This is a bit subtle when masks are
        // involved but I think the same logic applies.
        if masks.len() == self.num_levels() {
            debug_assert!(traversal.is_empty());
        } else {
            // Count any nodes left over in the traversal if it didn't traverse all levels,
            // ie. some bottom levels were ignored.
            //
            // I'm not sure if this is actually the behavior we want – it means that symbols
            // outside the range will be counted...
            //
            // Yeah, let's comment this out for now and leave this note here to decide later.
            //
            // for x in traversal.results() {
            //     counts[x.key] += x.val.end - x.val.start;
            // }
        }

        counts
    }

    // Returns the index of the first symbol less than or equal to `p` in the index range `range`.
    // todo: could we just do less than and use .. rather than ..=? doing ..=u32::max is kinda useless for this function...
    // ("First" here is based on sequence order; we will return the leftmost such index).
    // Implements the following logic:
    // selectFirstLeq = (arr, p, lo, hi) => {
    //   let i = arr.slice(lo, hi).findIndex((x) => x <= p);
    //   return i === -1 ? undefined : lo + i;
    // }
    // note: since the left extent of the target is always zero, we could optimize the containment checks.
    //
    pub fn select_first_less_than(&self, p: u32, range: Range<u32>) -> Option<u32> {
        let mut range = range; // index range
        let mut symbol = 0; // leftmost symbol in the currently-considered wavelet tree node
        let mut best = u32::max_value();
        let mut found = false;
        let target = 0..=p;

        // todo: select_[first/last[_[leq/geq].
        // The idea is to return the minimum select position across all the nodes that could
        // potentially contain the first symbol <= p.
        //
        // We find the first left node that is fully contained in the [0, p] symbol range,
        // and then we recurse into the right child if it is partly contained, and repeat.

        for (i, level) in self.levels.iter().enumerate() {
            if range.is_empty() {
                break;
            }

            let ignore_bits = self.num_levels() - i; // ignore all levels below this one when selecting
            let (left, mid, right) = level.splits(symbol); // value split points of left/right children

            // if this wavelet tree node is fully contained in the target range, update best and return.
            if target.contains_range(left..right) {
                let candidate = self.select_upwards(range.start, ignore_bits).unwrap();
                return Some(best.min(candidate));
            }

            let start = level.ranks(range.start);
            let end = level.ranks(range.end);

            // otherwise, we know that there are two possibilities:
            // 1. the left node is partly contained and the right node does not overlap the target
            // 2. the left node is fully contained and the right node may overlap the target
            if !target.contains_range(left..mid) {
                // we're in case 1, so refine our search range by going left
                range = start.0..end.0;
            } else {
                // we're in case 2, so update the best so far from the left child node if it is nonempty, then go right.
                if start.0 != end.0 {
                    // since this select is happening on the child level, un-ignore that level.
                    let candidate = self.select_upwards(start.0, ignore_bits - 1).unwrap();
                    best = best.min(candidate);
                    found = true;
                }
                // go right
                symbol += level.bit;
                range = level.nz + start.1..level.nz + end.1;
            }
        }

        if found {
            Some(best)
        } else {
            None
        }
    }

    pub fn locate_batch(&self, ranges: &[Range<u32>], symbols: &[u32]) -> Traversal<LocateBatch> {
        let iter = symbols.iter().flat_map(|symbol| {
            assert!(
                *symbol <= self.max_symbol,
                "symbol must not exceed max_symbol"
            );
            ranges
                .iter()
                .map(|range| (*symbol, 0, range.start, range.end))
        });
        let mut traversal = Traversal::new(iter.map(LocateBatch::new_tuple));
        for level in &self.levels {
            traversal.traverse(|xs, go| {
                for x in xs {
                    let (symbol, preceding_count) = (x.val.symbol, x.val.preceding_count);
                    let (start, end) = (level.ranks(x.val.start), level.ranks(x.val.end));
                    if symbol & level.bit == 0 {
                        go.left(x.val(LocateBatch {
                            symbol,
                            preceding_count,
                            start: start.0,
                            end: end.0,
                        }));
                    } else {
                        go.right(x.val(LocateBatch {
                            symbol,
                            preceding_count: preceding_count + end.0 - start.0,
                            start: level.nz + start.1,
                            end: level.nz + end.1,
                        }));
                    }
                }
            });
        }
        traversal
    }

    /// Return an iterator over levels from the high bit downwards, ignoring the
    /// bottom `ignore_bits` levels.
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

    // Wavelet matrix construction algorithm optimized for the case where we can afford to build a
    // dense histogram that counts the number of occurrences of each symbol. Heuristically,
    // this is roughly the case where the alphabet size does not exceed the number of data points.
    // Implements Algorithm 1 (seq.pc) from the paper "Practical Wavelet Tree Construction".
    fn build_bitvecs<T: BitVec>(
        data: Vec<u32>,
        num_levels: usize,
        bitvec_options: Option<<T::Builder as BitVecBuilder>::Options>,
    ) -> Vec<T> {
        assert!(data.len() <= u32::MAX as usize);
        let mut levels = vec![T::Builder::new(data.len() as u32); num_levels];
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

        levels
            .into_iter()
            .map(|level| {
                // apply options if any were passed in
                if let Some(o) = &bitvec_options {
                    level.options(o.clone())
                } else {
                    level
                }
            })
            .map(|level| level.build())
            .collect()
    }

    /// Wavelet matrix construction algorithm optimized for large alphabets.
    /// Returns an array of level bitvectors built from `data`.
    /// Handles the sparse case where the alphabet size exceeds the number of data points and
    /// building a histogram with an entry for each symbol is expensive.
    fn build_bitvecs_large_alphabet<T: BitVec>(
        mut data: Vec<u32>,
        num_levels: usize,
        bitvec_options: Option<<T::Builder as BitVecBuilder>::Options>,
    ) -> Vec<T> {
        assert!(data.len() <= u32::MAX as usize);
        let mut levels = Vec::with_capacity(num_levels);
        let max_level = num_levels - 1;

        // For each level, stably sort the datapoints by their bit value at that level.
        // Elements with a zero bit get sorted left, and elements with a one bits
        // get sorted right, which is effectvely a bucket sort with two buckets.
        let mut right = Vec::new();

        for l in 0..max_level {
            let level_bit = 1 << (max_level - l);
            let mut b = T::Builder::new(data.len() as u32);
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
            let mut b = T::Builder::new(data.len() as u32);
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
}

/// Type representing the state of an individual traversal path down the wavelet tree
/// during a count_symbol_range operation
#[derive(Copy, Clone, Debug)]
struct CountSymbolRange {
    acc: u32,   // mask accumulator for the levels traversed so far
    left: u32,  // leftmost symbol in the node
    start: u32, // index  range start
    end: u32,   // index range end
}

impl CountSymbolRange {
    fn new(acc: u32, left: u32, start: u32, end: u32) -> Self {
        CountSymbolRange {
            acc,
            left,
            start,
            end,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct LocateBatch {
    pub symbol: u32,          // leftmost symbol in the node
    pub preceding_count: u32, // number of symbols strictly less than this one
    pub start: u32,           // index range start
    pub end: u32,             // index range end
}

impl LocateBatch {
    fn new_tuple((symbol, preceding_count, start, end): (u32, u32, u32, u32)) -> Self {
        Self {
            symbol,
            preceding_count,
            start,
            end,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Counts {
    pub symbol: u32, // leftmost symbol in the node
    pub start: u32,  // index range start
    pub end: u32,    // index range end
}

impl Counts {
    fn new(symbol: u32, start: u32, end: u32) -> Self {
        Self { symbol, start, end }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panics;

    #[test]
    fn spot_test() {
        //    values:   1  3  3  2  7
        //    bits      -------------  high to low:
        //    level 0:  0  0  0  0  1  bit 2^2 = 4
        //    level 1:  0  1  1  1  1  bit 2^1 = 2
        //    level 2:  1  1  1  0  1  bit 2^0 = 1
        let data = vec![1, 3, 3, 2, 7];
        let len = data.len() as u32;
        let max_symbol = data.iter().max().copied().unwrap();
        let wm = WaveletMatrix::<DenseBitVec>::new(data.clone(), max_symbol, None);

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
            assert_eq!(wm.select(0..len, 0, 0, 0), None);
            assert_eq!(wm.select(0..len, 1, 0, 0), Some(0));
            assert_eq!(wm.select(0..len, 2, 0, 0), Some(3));
            assert_eq!(wm.select(0..len, 3, 0, 0), Some(1));
            assert_eq!(wm.select(0..len, 3, 1, 0), Some(2));
            assert_eq!(wm.select(0..len, 7, 0, 0), Some(4));
            assert_eq!(wm.select(0..len, 5, 0, 0), None);

            // select with 2 ignore_bits
            assert_eq!(wm.select(0..len, 0, 0, 2), Some(0));
            assert_eq!(wm.select(0..len, 1, 0, 2), Some(0));
            assert_eq!(wm.select(0..len, 2, 0, 2), Some(0));
            assert_eq!(wm.select(0..len, 3, 0, 2), Some(0));
            assert_eq!(wm.select(0..len, 3, 1, 2), Some(1));
            assert_eq!(wm.select(0..len, 7, 0, 2), Some(4));
            assert_eq!(wm.select(0..len, 5, 0, 2), Some(4));
            assert_eq!(wm.select(0..len, 100, 0, 2), None);

            // select with full ignore_bits
            assert_eq!(wm.select(0..len, 0, 0, wm.num_levels()), Some(0));
            assert_eq!(wm.select(0..len, 1, 0, wm.num_levels()), Some(0));
            assert_eq!(wm.select(0..len, 2, 0, wm.num_levels()), Some(0));
            assert_eq!(wm.select(0..len, 3, 0, wm.num_levels()), Some(0));
            assert_eq!(wm.select(0..len, 3, 1, wm.num_levels()), Some(1));
            assert_eq!(wm.select(0..len, 7, 0, wm.num_levels()), Some(0));
            assert_eq!(wm.select(0..len, 5, 0, wm.num_levels()), Some(0));
            assert_eq!(wm.select(0..len, 100, 0, wm.num_levels()), None);
        }

        {
            // select_last
            assert_eq!(wm.select_last(0..len, 0, 0, 0), None);
            assert_eq!(wm.select_last(0..len, 1, 0, 0), Some(0));
            assert_eq!(wm.select_last(0..len, 2, 0, 0), Some(3));
            assert_eq!(wm.select_last(0..len, 3, 0, 0), Some(2));
            assert_eq!(wm.select_last(0..len, 3, 1, 0), Some(1));
            assert_eq!(wm.select_last(0..len, 7, 0, 0), Some(4));
            assert_eq!(wm.select_last(0..len, 5, 0, 0), None);

            // select_last with 2 ignore_bits (just 1 not-ignored bit)
            assert_eq!(wm.select_last(0..len, 0, 0, 2), Some(3));
            assert_eq!(wm.select_last(0..len, 1, 0, 2), Some(3));
            assert_eq!(wm.select_last(0..len, 2, 0, 2), Some(3));
            assert_eq!(wm.select_last(0..len, 3, 0, 2), Some(3));
            assert_eq!(wm.select_last(0..len, 3, 1, 2), Some(2));
            assert_eq!(wm.select_last(0..len, 7, 0, 2), Some(4));
            assert_eq!(wm.select_last(0..len, 5, 0, 2), Some(4));
            assert_eq!(wm.select_last(0..len, 100, 0, 2), None);

            // select_last with full ignore_bits
            assert_eq!(wm.select_last(0..len, 0, 0, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(0..len, 1, 0, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(0..len, 2, 0, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(0..len, 3, 0, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(0..len, 3, 1, wm.num_levels()), Some(3));
            assert_eq!(wm.select_last(0..len, 7, 0, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(0..len, 5, 0, wm.num_levels()), Some(4));
            assert_eq!(wm.select_last(0..len, 100, 0, wm.num_levels()), None);
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
            assert_eq!(wm.quantile(0..len, 0), (1, 1));
            assert_eq!(wm.quantile(0..len, 1), (2, 1));
            assert_eq!(wm.quantile(0..len, 2), (3, 2));
            assert_eq!(wm.quantile(0..len, 3), (3, 2));
            assert_eq!(wm.quantile(0..len, 4), (7, 1));

            // multiplicity is within the reduced range
            assert_eq!(wm.quantile(1..2, 0), (3, 1));

            // quantile: check all values within a tighter range
            assert_eq!(wm.quantile(1..len - 1, 0), (2, 1));
            assert_eq!(wm.quantile(1..len - 1, 1), (3, 2));
            assert_eq!(wm.quantile(1..len - 1, 2), (3, 2));

            // quantile: out of bounds
            assert!(panics(|| wm.quantile(1..len - 1, 3,)));
        }

        {
            // preceding_count
            assert_eq!(wm.preceding_count(0..len, 0), 0);
            assert_eq!(wm.preceding_count(0..len, 1), 0);
            assert_eq!(wm.preceding_count(0..len, 2), 1);
            assert_eq!(wm.preceding_count(0..len, 3), 2);
            assert_eq!(wm.preceding_count(0..len, 4), 4);
            assert_eq!(wm.preceding_count(0..len, 5), 4);
            assert_eq!(wm.preceding_count(0..len, 7), 4);

            // preceding_count: symbol is beyond max_symbol
            assert!(panics(|| wm.preceding_count(0..len, max_symbol + 1)));
        }
    }
}
