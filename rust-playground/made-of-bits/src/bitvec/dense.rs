use crate::{
    bitbuf::BitBuf,
    bits::{
        basic_block_index, basic_block_offset, one_mask, select1, BASIC_BLOCK_BITS,
        BASIC_BLOCK_SIZE,
    },
    bitvec::{BitVec, BitVecBuilder},
};

// todo
// - figure out how to optionally use a PaddedBitBuf – Buf trait + type parameter?
// - do a pass over the code and convert it to a more Rust-like style: the current impls are fairly direct ports from JavaScript.

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
    basic_blocks_per_rank1_sample_pow2: u32,
}

impl DenseBitVec {
    /// `buf`: bit buffer containing the underlying bit data
    /// `rank1_samples_pow2`: power of 2 of the rank1 sample rate
    /// `select_samples_pow2`: power of 2 of the select sample rate for both select0 and select1
    pub(crate) fn new(buf: BitBuf, rank1_samples_pow2: u32, select_samples_pow2: u32) -> Self {
        assert!(BASIC_BLOCK_BITS <= rank1_samples_pow2 && rank1_samples_pow2 < 32);
        assert!(BASIC_BLOCK_BITS <= select_samples_pow2 && select_samples_pow2 < 32);

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
        // A rank sample `rank1_samples[i]` tells us about the basic block `buf.blocks[i << (srPow2 - BASIC_BLOCK_BITS)]`.
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

        let basic_blocks_per_rank1_sample = rank1_sample_rate >> BASIC_BLOCK_BITS;

        let max_block_index = buf.num_blocks().saturating_sub(1);
        for block_index in 0..buf.num_blocks() {
            let block = buf.get_block(block_index);
            if block_index % basic_blocks_per_rank1_sample == 0 {
                rank1_samples.push(cumulative_ones);
            }

            let mut block_ones = block.count_ones();
            let mut block_zeros = BASIC_BLOCK_SIZE - block_ones;

            // Don't count trailing ones or zeros in the final data block towards the 0/1 count
            if block_index == max_block_index {
                let num_non_trailing_bits = BASIC_BLOCK_SIZE - buf.num_trailing_bits();
                let trailing_bits = block & !one_mask(num_non_trailing_bits);
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
            cumulative_bits = cumulative_bits.saturating_add(BASIC_BLOCK_SIZE);
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
            basic_blocks_per_rank1_sample_pow2: rank1_samples_pow2 - BASIC_BLOCK_BITS,
        }
    }

    // `n` - we are looking for the n-th bit of the particular kind (1-bit or 0-bit)
    // `sampleRate` - power of 2 of the select sample rate
    // `samples` - array of samples
    fn select_sample(n: u32, samples: &Box<[u32]>, sample_rate: u32) -> (u32, u32) {
        let sample_index = n >> sample_rate;
        let sample = samples[sample_index as usize];
        let mask = const { one_mask(BASIC_BLOCK_BITS) };
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
        return (preceding_count, basic_block_index(cumulative_bits) as u32);
    }
}

impl BitVec for DenseBitVec {
    type Builder = DenseBitVecBuilder;

    fn rank1(&self, bit_index: u32) -> u32 {
        if bit_index >= self.universe_size() {
            return self.num_ones();
        }

        // todo: investigate whether we can provide a 'hint' argument of a start block
        // that would allow us to skip the rank/select memory fetches if querying
        // another 1-bit close by. As another way to do a 'batch' operation for a
        // sorted input.

        // Start with the prefix count from the rank block
        let rank_index = bit_index >> self.rank1_samples_pow2; // todo: why can we inject a +1 here without tests failing?
        let mut count = self.rank1_samples[rank_index as usize];
        let mut rank_basic_block_index = rank_index << self.basic_blocks_per_rank1_sample_pow2;
        let last_basic_block_index = basic_block_index(bit_index) as u32;

        // note: this select-block-skipper actually somewhat slows down some wavelet matrix ops.

        // Scan any intervening select blocks to skip past multiple basic blocks at a time.
        //
        // Synthesize a fictitious initial select sample located squarely at the position
        // designated by the rank sample.
        let select_sample_rate = 1 << self.select1_samples_pow2;
        let select_basic_block_index = rank_basic_block_index;
        let select_preceding_count = count;
        let mut select_count = select_preceding_count + select_sample_rate;
        while select_count < self.num_ones() && select_basic_block_index < last_basic_block_index {
            let (select_preceding_count, select_basic_block_index) = DenseBitVec::select_sample(
                select_count,
                &self.select1_samples,
                self.select1_samples_pow2,
            );
            if select_basic_block_index >= last_basic_block_index {
                break;
            }
            count = select_preceding_count;
            rank_basic_block_index = select_basic_block_index;
            select_count += select_sample_rate;
        }

        // Increment the count by the number of ones in every subsequent block
        for i in rank_basic_block_index..last_basic_block_index {
            count += self.buf.get_block(i).count_ones();
        }

        // Count any 1-bits in the last block up to `bit_index`
        let bit_offset = basic_block_offset(bit_index);
        let masked_block = self.buf.get_block(last_basic_block_index) & one_mask(bit_offset);
        count += masked_block.count_ones();
        count
    }

    fn select1(&self, n: u32) -> Option<u32> {
        if n >= self.num_ones() {
            return None;
        }

        // Grab the basic block and count information from the select sample
        let (mut count, mut basic_block_index) =
            DenseBitVec::select_sample(n, &self.select1_samples, self.select1_samples_pow2);
        assert!(count <= n);
        // assert the previous rank index is less than the number of rank samples
        debug_assert!(
            (basic_block_index >> self.basic_blocks_per_rank1_sample_pow2)
                < self.rank1_samples.len() as u32
        );

        // Scan any intervening rank blocks to skip past multiple basic blocks at a time
        let mut rank_index = (basic_block_index >> self.basic_blocks_per_rank1_sample_pow2) + 1;
        let num_rank_samples = self.rank1_samples.len() as u32;
        while rank_index < num_rank_samples {
            let next_count = self.rank1_samples[rank_index as usize];
            if next_count > n {
                break;
            }
            count = next_count;
            basic_block_index = rank_index << self.basic_blocks_per_rank1_sample_pow2;
            rank_index += 1;
        }

        // Scan basic blocks until we find the one that contains the n-th 1-bit
        let mut basic_block = 0;
        assert!(basic_block_index < self.buf.num_blocks()); // the index is in-bounds for the first iteration
        while basic_block_index < self.buf.num_blocks() {
            basic_block = self.buf.get_block(basic_block_index);
            let next_count = count + basic_block.count_ones();
            if next_count > n {
                break;
            }
            count = next_count;
            basic_block_index += 1;
        }

        // Compute and return its bit index
        let basic_block_bit_index = basic_block_index << BASIC_BLOCK_BITS;
        let bit_offset = select1(basic_block, n - count).unwrap_or(0);
        Some(basic_block_bit_index + bit_offset)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        // This implementation is adapted from select1.
        if n >= self.num_zeros() {
            return None;
        }

        // Grab the basic block and count information from the select sample
        let (mut count, mut basic_block_index) =
            DenseBitVec::select_sample(n, &self.select0_samples, self.select0_samples_pow2);
        assert!(count <= n);
        // assert the previous rank index is less than the number of rank samples
        debug_assert!(
            (basic_block_index >> self.basic_blocks_per_rank1_sample_pow2)
                < self.rank1_samples.len() as u32
        );

        // Scan any intervening rank blocks to skip past multiple basic blocks at a time
        let mut rank_index = (basic_block_index >> self.basic_blocks_per_rank1_sample_pow2) + 1;
        let num_rank_samples = self.rank1_samples.len() as u32;
        while rank_index < num_rank_samples {
            let next_count =
                (rank_index << self.rank1_samples_pow2) - self.rank1_samples[rank_index as usize];
            if next_count > n {
                break;
            }
            count = next_count;
            basic_block_index = rank_index << self.basic_blocks_per_rank1_sample_pow2;
            rank_index += 1;
        }

        // Scan basic blocks until we find the one that contains the n-th 0-bit
        let mut basic_block = 0;
        assert!(basic_block_index < self.buf.num_blocks()); // the index is in-bounds for the first iteration
        while basic_block_index < self.buf.num_blocks() {
            basic_block = self.buf.get_block(basic_block_index);
            let next_count = count + basic_block.count_zeros();
            if next_count > n {
                break;
            }
            count = next_count;
            basic_block_index += 1;
        }

        // Compute and return its bit index
        let basic_block_bit_index = basic_block_index << BASIC_BLOCK_BITS;
        let bit_offset = select1(!basic_block, n - count).unwrap_or(0);
        Some(basic_block_bit_index + bit_offset)
    }

    fn universe_size(&self) -> u32 {
        self.buf.universe_size()
    }

    fn num_ones(&self) -> u32 {
        self.num_ones
    }
}

#[derive(Default, Clone)]
pub struct DenseBitVecOptions {
    rank1_samples_pow2: Option<u32>,
    select_samples_pow2: Option<u32>,
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

    fn new(universe_size: u32) -> Self {
        Self {
            buf: BitBuf::new(universe_size),
            options: Default::default(),
        }
    }

    fn options(mut self, options: Self::Options) -> Self {
        self.options = options;
        self
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

    #[test]
    fn bitvec_interface() {
        test_bitvec_builder::<DenseBitVecBuilder>();
    }
}
