use crate::bitblock::BitBlock;
use crate::{
    bitbuf,
    bitbuf::BitBuf,
    bits,
    bits::{one_mask, select64_checked},
    bitvec::{BitVec, BitVecBuilder},
};

/// Implements a BitVec based on a dense bit buffer.
/// Takes 1 bit per 1-bit, plus overhead based on rank
/// and select samples which are used to accelerate queries.
/// The anount of overhead depends on the rank and select sampling
/// rate. By default, rank1 samples take ~3% of the space of the
/// data in the bit buffer, and select samples add another ~3% since
/// together the select0 and select1 samples are taken once every 2^10 1-bits.
#[derive(Clone)]
pub struct DenseBitVec {
    buf: BitBuf,
    num_ones: u32,
    rank1_samples_pow2: u32,
    select0_samples_pow2: u32,
    select1_samples_pow2: u32,
    rank1_samples: Box<[u32]>,
    select0_samples: Box<[u32]>,
    select1_samples: Box<[u32]>,
    /// A buf block is a block of the BitBuf.
    buf_blocks_per_rank1_sample_pow2: u32,
}

impl DenseBitVec {
    /// `buf`: bit buffer containing the underlying bit data
    /// `rank1_samples_pow2`: power of 2 of the rank1 sample rate
    /// `select_samples_pow2`: power of 2 of the select sample rate for both select0 and select1
    pub(crate) fn new(buf: BitBuf, rank1_samples_pow2: u32, select_samples_pow2: u32) -> Self {
        assert!(bitbuf::Block::BITS_LOG2 <= rank1_samples_pow2 && rank1_samples_pow2 < 32);
        assert!(bitbuf::Block::BITS_LOG2 <= select_samples_pow2 && select_samples_pow2 < 32);

        // Sample rank1 samples every `rank1_sampling_rate` bits
        let rank1_sample_rate = 1 << rank1_samples_pow2;

        // Sample select1 samples every `select1_sampling_rate` 1-bits
        let select1_sample_rate = 1 << select_samples_pow2;

        // Sample select0 samples every `select0_sampling_rate` 0-bits
        let select0_sample_rate = 1 << select_samples_pow2;

        // Each rank sample identifies a particular basic block.
        //

        // Rank samples are sampled every `rank1_sampling_rate` bits, where `rank1_sampling_rate` is a positive multiple of
        // the bit width of a basic block. For example, if `rank1_sampling_rate` is 64 and the basic
        // block width is 32, then the rank samples will tell us about the 0th, 2nd, 4th, 6th, ... basic block.
        //
        // A rank sample `rank1_samples[i]` tells us about the basic block `buf.blocks[i << (srPow2 - bitbuf::Block::BITS)]`.
        //
        // If `rank1_samples[i] has value `v`, this means that there are `v` 1-bits preceding that basic block.
        // Rank samples represent the number of 1-bits up to but not including a basic block.
        let mut rank1_samples = Vec::new();

        // Each select1 sample identifies a particular basic block.
        //
        // Select samples are sampled every `select1_sampling_rate` 1-bits, where `rank1_sampling_rate` is a positive multiple of
        // the bit width of a basic block. Unlike rank blocks, which start sampling from 0 (representing the
        // `rank1_sampling_rate*i + 0`-th bits), select blocks start sampling from 1, and thus represent the
        // `select1_sampling_rate*i + 1`-th bits.
        // For example, if `select1_sampling_rate` is 64, then the select1 samples will identify the basic blocks
        // that contain the 0+1 = 1st, 64+1 = 65th, 2*64+1 = 129th, 3*64+1 = 193rd, ... bits.
        // Since the sampling rate is a positive multiple of the basic block, two select blocks will never point
        // to the same basic block.
        let mut select1_samples = Vec::new();
        let mut select0_samples = Vec::new();

        // Select1 samples represent the number of 1-bits up to but not including a basic block.
        // For example, if `select1_sampling_rate`
        // is 64, then the select1 samples will tell us about the basic blocks containing the 1st
        // A select sample `select1_samples[i]` tells us about the basic block that contains the
        // `selectSamplingRate * i + 1`-th 1-bit.
        let mut cumulative_ones = 0; // 1-bits preceding the current raw block
        let mut cumulative_bits = 0; // bits preceding the current raw block
        let mut zeros_threshold = 0; // take a select0 sample at the (zerosThreshold+1)th 1-bit
        let mut ones_threshold = 0; // take a select1 sample at the (onesThreshold+1)th 1-bit

        let buf_blocks_per_rank1_sample = rank1_sample_rate >> bitbuf::Block::BITS_LOG2;

        let max_block_index = buf.num_blocks().saturating_sub(1);
        for block_index in 0..buf.num_blocks() {
            let block = buf.block(block_index);
            if block_index % buf_blocks_per_rank1_sample == 0 {
                rank1_samples.push(cumulative_ones);
            }

            let mut block_ones = block.count_ones();
            let mut block_zeros = bitbuf::Block::BITS - block_ones;

            // Don't count trailing ones or zeros in the final data block towards the 0/1 count
            if block_index == max_block_index {
                let num_non_trailing_bits = bitbuf::Block::BITS - buf.num_trailing_bits();
                let trailing_bits = block & !one_mask::<bitbuf::Block>(num_non_trailing_bits);
                let trailing_bits_ones = trailing_bits.count_ones();
                let trailing_bits_zeros = buf.num_trailing_bits() - trailing_bits_ones;

                block_ones -= trailing_bits_ones;
                block_zeros -= trailing_bits_zeros;
            }

            let cumulative_zeros = cumulative_bits - cumulative_ones;

            // Sample 1-bits for the select1 index
            if cumulative_ones + block_ones > ones_threshold {
                // Take a select1 sample, which consists of two parts:
                // 1. The cumulative number of bits preceding this basic block, ie. the left-shifted block index.
                //    This is `cumulative_bits`, defined above, and is stored in the high bits.
                // 2. A correction factor storing the number of 1-bits preceding the (ss1 * i + 1)-th 1-bit within this
                //    basic block, which we can use to determine the number of 1-bits preceding this basic block.
                //    Effectively, this is a way for us to store samples that are slightly offset from the strictly
                //    regular select sampling scheme, enabling us to keep the select samples aligned to basic blocks.
                //    This is `correction`, and is stored in the low bits.
                let correction = ones_threshold - cumulative_ones;
                // Since cumulative_bits is a multiple of the basic block size,
                // these two values should never overlap in their bit ranges.
                debug_assert!(cumulative_bits & correction == 0);
                // Add the select sample and bump the onesThreshold.
                select1_samples.push(cumulative_bits | correction);
                ones_threshold = ones_threshold.saturating_add(select1_sample_rate);
            }

            // Sample 0-bits for the select0 index.
            // This `if` block has the same structure as the one above which samples 1-bits.
            if cumulative_zeros + block_zeros > zeros_threshold {
                let correction = zeros_threshold - cumulative_zeros;
                debug_assert!(cumulative_bits & correction == 0);
                select0_samples.push(cumulative_bits | correction);
                zeros_threshold = zeros_threshold.saturating_add(select0_sample_rate);
            }

            cumulative_ones += block_ones;
            cumulative_bits = cumulative_bits.saturating_add(bitbuf::Block::BITS);
        }

        Self {
            buf,
            num_ones: cumulative_ones,
            rank1_samples_pow2,
            select0_samples_pow2: select_samples_pow2,
            select1_samples_pow2: select_samples_pow2,
            rank1_samples: rank1_samples.into(),
            select0_samples: select0_samples.into(),
            select1_samples: select1_samples.into(),
            buf_blocks_per_rank1_sample_pow2: rank1_samples_pow2 - bitbuf::Block::BITS_LOG2,
        }
    }

    // `n` - we are looking for the n-th bit of the particular kind (1-bit or 0-bit)
    // `sampleRate` - power of 2 of the select sample rate
    // `samples` - array of samples
    fn select_sample(n: u32, samples: &Box<[u32]>, sample_rate: u32) -> (u32, u32) {
        let sample_index = n >> sample_rate;
        let sample = samples[sample_index as usize];
        let mask = one_mask::<u32>(bitbuf::Block::BITS_LOG2);
        // The cumulative number of bits preceding the identified basic block,
        // ie. the left-shifted block index of that block.
        let cumulative_bits = sample & !mask; // high bits

        // NOTE: The references to 1-bits below are written from the perspective of select1.
        // If using this function for select zero, think of "1-bit" as "0-bit".

        // The number of 1-bits in the identified basic block preceding the (select1SampleRate*i+1)-th 1-bit
        let correction = sample & mask; // low bits

        // number of 1-bits preceding the identified basic block.
        // The first term tells us the number of 1-bits preceding this select sample,
        // since the k-th sample represents the (k*sr + 1)-th bit and this tells us the (k*sr)-th
        // The second term allows us to identify how may 1-bits precede the basic block containing
        // the bit identified by this select sample.
        let preceding_count = (sample_index << sample_rate) - correction;
        return (
            preceding_count,
            bitbuf::Block::block_index(cumulative_bits) as u32,
        );
    }

    /// Returns a (count, start_index) pair to be used as the starting position
    /// for a linear search through bit blocks when computing rank1 using rank1_hinted.
    fn rank1_hint(&self, bit_index: u32) -> (u32, usize) {
        // Start with the prefix count from the rank block
        let rank_index = bit_index >> self.rank1_samples_pow2;
        let mut count = self.rank1_samples[rank_index as usize];
        let mut start_index = rank_index << self.buf_blocks_per_rank1_sample_pow2;
        (count, start_index as usize)
    }

    /// Compute rank1, with an optional hint from a preceding call to rank1_hinted which,
    /// if provided, will be used instead of computing such a hint from the rank blocks
    /// using `rank1_hint`. When searching for closely spaced bit indices providing a hint
    /// can speed up processing significantly since it reduces the amount of memory traffic.
    ///
    /// If a hint is provided, then a linear search will be conducted from that starting position
    /// until the desired bit index is reached. This can slow performance if the hint is for a
    /// distant bit position.
    fn rank1_hinted(&self, bit_index: u32, hint: Option<(u32, usize)>) -> (u32, (u32, usize)) {
        if bit_index >= self.universe_size() {
            // the hint does we return here does not matter since all queries that
            // use the hint will fall into this branch.
            return (self.num_ones(), (0, 0));
        }

        let (mut count, start_index) = hint.unwrap_or_else(|| self.rank1_hint(bit_index));
        let last_index = bitbuf::Block::block_index(bit_index);

        // Increment the count by the number of ones in every subsequent block
        let blocks = self.buf.blocks();
        for block in &blocks[start_index..last_index] {
            count += block.count_ones();
        }

        // Count any 1-bits in the last block up to `bit_index`
        let bit_offset = bitbuf::Block::block_bit_index(bit_index);
        let masked_block = blocks[last_index] & one_mask::<bitbuf::Block>(bit_offset);
        (count + masked_block.count_ones(), (count, last_index))
    }

    // Note: This code used to be part of `rank1_hinted` but was removed since it did not improve performance.
    //
    // Scan any intervening select blocks to skip past multiple basic blocks at a time.
    //
    // Synthesize a fictitious initial select sample located squarely at the position
    // designated by the rank sample.
    //
    // Note: When rank samples are sufficiently close (eg. rank_samples_pow2 = 2^10),
    // this slows rank queries down rather than speeding them up (confirmed with Criterion
    // benchmarks.) Keeping this code here but commented-out since there could be value in
    // using this technique in the future.
    //
    // let select_sample_rate = 1 << self.select1_samples_pow2;
    // let select_buf_block_index = start_index;
    // let select_preceding_count = count;
    // let mut select_count = select_preceding_count + select_sample_rate;
    // while select_count < self.num_ones() && select_buf_block_index < last_index {
    //     let (select_preceding_count, select_buf_block_index) = DenseBitVec::select_sample(
    //         select_count,
    //         &self.select1_samples,
    //         self.select1_samples_pow2,
    //     );
    //     if select_buf_block_index >= last_index {
    //         break;
    //     }
    //     count = select_preceding_count;
    //     start_index = select_buf_block_index;
    //     select_count += select_sample_rate;
    // }

    /// `select1` with an optional `(count, buf_block_idx)` hint that lets nearby
    /// sorted queries resume their linear scan instead of re-seeding from the select
    /// sample table. Returns `Some((position, new_hint))` on success.
    ///
    /// Both the hint-taken and sample-seeded paths share the rank-block fast-forward
    /// loop below; without it, a stale hint would crawl one block at a time across a
    /// gap that the sample path leaps over in one rank-bucket step. (Benches showed
    /// this fix as the difference between a hint-batch slowdown and a 1.4–1.9× win
    /// on moderate-density sparse-query workloads.)
    ///
    /// Possible future refinements, *not implemented* — measure before adopting:
    ///
    /// - Skip the hint when no fresher than a fresh sample lookup, via the cheap
    ///   `hint.count >= (n >> samples_pow2) << samples_pow2` check.
    /// - Split into `_checked` / `_unchecked` cores so `select1_batch` skips the
    ///   per-query `Option` match.
    pub fn select1_hinted(
        &self,
        n: u32,
        hint: Option<(u32, u32)>,
    ) -> Option<(u32, (u32, u32))> {
        if n >= self.num_ones() {
            return None;
        }
        let (mut count, mut buf_block_index) = match hint {
            Some(h) if h.0 <= n => h,
            _ => Self::select_sample(n, &self.select1_samples, self.select1_samples_pow2),
        };
        // Fast-forward through whole rank-sample buckets when possible.
        let mut rank_index = (buf_block_index >> self.buf_blocks_per_rank1_sample_pow2) + 1;
        while rank_index < self.rank1_samples.len() as u32 {
            let next_count = self.rank1_samples[rank_index as usize];
            if next_count > n {
                break;
            }
            count = next_count;
            buf_block_index = rank_index << self.buf_blocks_per_rank1_sample_pow2;
            rank_index += 1;
        }
        // Find the basic block containing the n-th 1-bit.
        let mut buf_block = 0;
        while buf_block_index < self.buf.num_blocks() {
            buf_block = self.buf.block(buf_block_index);
            let next_count = count + buf_block.count_ones();
            if next_count > n {
                break;
            }
            count = next_count;
            buf_block_index += 1;
        }
        let bit_offset = select64_checked(buf_block, n - count).unwrap_or(0);
        let pos = (buf_block_index << bitbuf::Block::BITS_LOG2) + bit_offset;
        Some((pos, (count, buf_block_index)))
    }

    /// Run `select1` for every value in `indices`, writing the positions back in
    /// place. Out-of-range indices (`>= num_ones()`) become `u32::MAX`.
    ///
    /// Threads a hint between queries so nearby ones can skip the sample lookup. The
    /// hint pays off when input is monotone non-decreasing; unsorted input still
    /// produces correct results (the `hint.count <= n` check falls back to a sample
    /// lookup), just without the speedup.
    pub fn select1_batch(&self, indices: &mut [u32]) {
        let mut hint = None;
        for i in indices {
            match self.select1_hinted(*i, hint) {
                Some((pos, h)) => {
                    hint = Some(h);
                    *i = pos;
                }
                None => *i = u32::MAX,
            }
        }
    }

    /// Ascending positions of every 1-bit.
    pub fn ones(&self) -> Select1Range<'_> {
        self.select1_range(0..self.universe_size())
    }

    /// Ascending positions of 1-bits in `[range.start, range.end)`. Walks blocks
    /// sequentially, yielding the lowest set bit of each via `block &= block - 1`.
    /// The shared per-block residual is why this is an iterator and not a slice.
    pub fn select1_range(&self, range: std::ops::Range<u32>) -> Select1Range<'_> {
        Select1Range::new(self, range)
    }
}

/// Iterator over the 1-bit positions in a [`DenseBitVec`] range, ascending.
pub struct Select1Range<'a> {
    bv: &'a DenseBitVec,
    /// Currently-masked block; the lowest set bit gets yielded next.
    block: bitbuf::Block,
    block_idx: u32,
    end_block: u32,
    /// Mask applied to the final block to clip bits at or after `range.end`.
    end_mask: bitbuf::Block,
}

impl<'a> Select1Range<'a> {
    fn new(bv: &'a DenseBitVec, range: std::ops::Range<u32>) -> Self {
        let universe = bv.universe_size();
        let start = range.start.min(universe);
        let end = range.end.min(universe);
        if start >= end || bv.buf.num_blocks() == 0 {
            return Self {
                bv,
                block: 0,
                block_idx: 1,
                end_block: 0,
                end_mask: 0,
            };
        }
        let block_idx = bitbuf::Block::block_index(start) as u32;
        let end_block = bitbuf::Block::block_index(end - 1) as u32;
        let end_bit = bitbuf::Block::block_bit_index(end);
        let end_mask = if end_bit == 0 {
            bitbuf::Block::MAX
        } else {
            one_mask::<bitbuf::Block>(end_bit)
        };
        let mut iter = Self {
            bv,
            block: 0,
            block_idx,
            end_block,
            end_mask,
        };
        // Pre-mask the first block: clip bits below `start`.
        iter.block = iter.load_block(block_idx)
            & !one_mask::<bitbuf::Block>(bitbuf::Block::block_bit_index(start));
        iter
    }

    /// Apply the trailing-bits and end-block masks to `block_idx`'s bits.
    fn load_block(&self, block_idx: u32) -> bitbuf::Block {
        let mut block = self.bv.buf.block(block_idx);
        if block_idx + 1 == self.bv.buf.num_blocks() && self.bv.buf.num_trailing_bits() > 0 {
            let num_valid = bitbuf::Block::BITS - self.bv.buf.num_trailing_bits();
            block &= one_mask::<bitbuf::Block>(num_valid);
        }
        if block_idx == self.end_block {
            block &= self.end_mask;
        }
        block
    }
}

impl Iterator for Select1Range<'_> {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        loop {
            if self.block != 0 {
                let tz = self.block.trailing_zeros();
                self.block &= self.block - 1;
                return Some((self.block_idx << bitbuf::Block::BITS_LOG2) + tz);
            }
            if self.block_idx >= self.end_block {
                return None;
            }
            self.block_idx += 1;
            self.block = self.load_block(self.block_idx);
        }
    }
}

impl BitVec for DenseBitVec {
    type Builder = DenseBitVecBuilder;

    fn rank1(&self, bit_index: u32) -> u32 {
        self.rank1_hinted(bit_index, None).0
    }

    fn select1(&self, n: u32) -> Option<u32> {
        self.select1_hinted(n, None).map(|(pos, _)| pos)
    }

    // todo: could we provide hinted selects? could be useful eg. in the sparse vector where
    // we want to call select0 with n and n+1.
    fn select0(&self, n: u32) -> Option<u32> {
        // This implementation is adapted from select1.
        if n >= self.num_zeros() {
            return None;
        }

        // Grab the basic block and count information from the select sample
        let (mut count, mut buf_block_index) =
            DenseBitVec::select_sample(n, &self.select0_samples, self.select0_samples_pow2);
        assert!(count <= n);
        // assert the previous rank index is less than the number of rank samples
        debug_assert!(
            (buf_block_index >> self.buf_blocks_per_rank1_sample_pow2)
                < self.rank1_samples.len() as u32
        );

        // Scan any intervening rank blocks to skip past multiple basic blocks at a time
        let mut rank_index = (buf_block_index >> self.buf_blocks_per_rank1_sample_pow2) + 1;
        let num_rank_samples = self.rank1_samples.len() as u32;
        while rank_index < num_rank_samples {
            let next_count =
                (rank_index << self.rank1_samples_pow2) - self.rank1_samples[rank_index as usize];
            if next_count > n {
                break;
            }
            count = next_count;
            buf_block_index = rank_index << self.buf_blocks_per_rank1_sample_pow2;
            rank_index += 1;
        }

        // Scan basic blocks until we find the one that contains the n-th 0-bit
        let mut buf_block = 0;
        assert!(buf_block_index < self.buf.num_blocks()); // the index is in-bounds for the first iteration
        while buf_block_index < self.buf.num_blocks() {
            buf_block = self.buf.block(buf_block_index);
            let next_count = count + bitbuf::Block::count_zeros(buf_block);
            if next_count > n {
                break;
            }
            count = next_count;
            buf_block_index += 1;
        }

        // Compute and return its bit index
        let buf_block_bit_index = buf_block_index << bitbuf::Block::BITS_LOG2;
        let bit_offset = select64_checked(!buf_block, n - count).unwrap_or(0);
        Some(buf_block_bit_index + bit_offset)
    }

    fn universe_size(&self) -> u32 {
        self.buf.universe_size()
    }

    fn num_ones(&self) -> u32 {
        self.num_ones
    }

    fn rank1_batch(&self, bit_indices: &mut [u32]) {
        let chunks = bit_indices.chunk_by_mut(|a, b| {
            // note: we could instead measure the distance in terms of actual rank blocks.
            //       this is an interesting parameter to play with.
            let dist = (b - a) >> self.rank1_samples_pow2;
            dist <= 1
        });
        for chunk in chunks {
            let mut hint = None;
            for i in chunk {
                let result = self.rank1_hinted(*i, hint);
                hint = Some(result.1);
                *i = result.0;
            }
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct DenseBitVecOptions {
    pub rank1_samples_pow2: Option<u32>,
    pub select_samples_pow2: Option<u32>,
}

#[derive(Clone)]
pub struct DenseBitVecBuilder {
    buf: BitBuf,
    options: DenseBitVecOptions,
}

impl BitVecBuilder for DenseBitVecBuilder {
    type Target = DenseBitVec;
    /// (rank1_samples_pow2, select_samples_pow2)
    type Options = DenseBitVecOptions;

    fn new(universe_size: u32, options: Self::Options) -> Self {
        Self {
            buf: BitBuf::new(universe_size),
            options,
        }
    }

    fn build(self) -> DenseBitVec {
        // todo: compress to padded bit buf if favorable?
        DenseBitVec::new(
            self.buf,
            self.options.rank1_samples_pow2.unwrap_or(10),
            self.options.select_samples_pow2.unwrap_or(10),
        )
    }

    fn one(&mut self, bit_index: u32) {
        assert!(bit_index < self.buf.universe_size());
        self.buf.set_one(bit_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec::test::*;
    use expect_test::expect;

    #[test]
    fn bitvec_interface() {
        test_bitvec_builder::<DenseBitVecBuilder>();
    }

    /// `select1_batch`, `ones`, and `select1_range` must agree with repeated `select1`
    /// across arbitrary (universe, ones) configurations.
    #[test]
    fn select1_iterators_and_batch() {
        use arbtest::arbtest;
        use crate::bitvec::BitVec;
        arbtest(|u| {
            let universe = u.int_in_range(0u32..=512)?;
            let mut ones: Vec<u32> = Vec::new();
            for _ in 0..u.int_in_range(0u32..=universe.max(1))? {
                if universe == 0 {
                    break;
                }
                ones.push(u.int_in_range(0..=universe - 1)?);
            }
            ones.sort_unstable();
            ones.dedup();
            let bv = DenseBitVecBuilder::from_ones(universe, Default::default(), &ones);

            // ones() yields the same positions as select1(0..num_ones).
            let from_iter: Vec<u32> = bv.ones().collect();
            assert_eq!(from_iter, ones, "ones() mismatch");

            // select1_range respects the bounds.
            if !ones.is_empty() {
                let a = u.int_in_range(0u32..=universe)?;
                let b = u.int_in_range(0u32..=universe)?;
                let range = a.min(b)..a.max(b);
                let expected: Vec<u32> =
                    ones.iter().copied().filter(|p| range.contains(p)).collect();
                let got: Vec<u32> = bv.select1_range(range).collect();
                assert_eq!(got, expected, "select1_range mismatch");
            }

            // select1_batch matches repeated select1.
            let mut indices: Vec<u32> = (0..bv.num_ones()).collect();
            let expected: Vec<u32> = indices.iter().map(|&i| bv.select1(i).unwrap()).collect();
            bv.select1_batch(&mut indices);
            assert_eq!(indices, expected, "select1_batch mismatch");
            Ok(())
        });
    }

    /// Snapshot the bit-pattern + sample-table layout of a small DenseBitVec.
    /// Acts as a golden test for the on-disk-equivalent representation: any
    /// change in the rank/select sample emission strategy or block layout shows
    /// up here. Update with `UPDATE_EXPECT=1 cargo test snapshot_layout`.
    #[test]
    fn snapshot_layout() {
        let mut b = DenseBitVecBuilder::new(70, Default::default());
        for i in [0, 31, 32, 68] {
            b.one(i);
        }
        let bv = b.build();
        let snapshot = format!(
            "universe_size={}\nnum_ones={}\nblocks={:?}\nrank1_samples={:?}\nselect1_samples={:?}\nselect0_samples={:?}",
            bv.universe_size(),
            bv.num_ones(),
            bv.buf.blocks(),
            bv.rank1_samples,
            bv.select1_samples,
            bv.select0_samples,
        );
        expect![[r#"
            universe_size=70
            num_ones=4
            blocks=[6442450945, 16]
            rank1_samples=[0]
            select1_samples=[0]
            select0_samples=[0]"#]]
        .assert_eq(&snapshot);
    }
}
