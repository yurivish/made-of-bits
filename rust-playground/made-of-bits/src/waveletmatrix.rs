use crate::bitvec::BitVec;
use crate::bitvec::BitVecBuilder;
use crate::{
    bits::reverse_low_bits,
    bitvec::dense::{DenseBitVec, DenseBitVecBuilder},
};

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

impl<V: BitVec> WaveletMatrix<V> {
    pub fn new(data: Vec<u32>, max_symbol: u32) -> WaveletMatrix<DenseBitVec> {
        // Equivalent to max(1, ceil(log2(alphabet_size))), which ensures
        // that we always have at least one level even if all symbols are 0.
        let num_levels = (max_symbol + 1).ilog2().max(1) as u32;

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

    pub fn from_bitvecs(levels: Vec<V>, max_symbol: u32) -> WaveletMatrix<V> {
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
