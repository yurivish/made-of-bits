use crate::{
    bitbuf::{BitBuf, PaddedBitBuf},
    bits::{one_mask, BASIC_BLOCK_BITS, BASIC_BLOCK_SIZE},
};

// todo
// - figure out how to optionally use a PaddedBitBuf – Buf trait + type parameter?
struct DenseBitVec {
    // needs densebitvec actually, for the high bits.
    buf: BitBuf,
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
    fn new(buf: BitBuf, rank1_samples_pow2: u32, select_samples_pow2: u32) -> Self {
        assert!((BASIC_BLOCK_BITS..32).contains(&rank1_samples_pow2));
        assert!((BASIC_BLOCK_BITS..32).contains(&select_samples_pow2));

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

        let mut cumulative_ones: u32 = 0; // 1-bits preceding the current raw block
        let mut cumulative_bits: u32 = 0; // bits preceding the current raw block
        let mut zeros_threshold: u32 = 0; // take a select0 sample at the (zerosThreshold+1)th 1-bit
        let mut ones_threshold: u32 = 0; // take a select1 sample at the (onesThreshold+1)th 1-bit

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
                ones_threshold += select1_sample_rate;
            }

            // Sample 0-bits for the select0 index.
            // This `if` block has the same structure as the one above which samples 1-bits.
            if cumulative_zeros + block_zeros > zeros_threshold {
                let correction = zeros_threshold - cumulative_bits;
                debug_assert!(cumulative_bits & correction == 0);
                select0_samples.push(cumulative_bits | correction);
                zeros_threshold += select0_sample_rate;
            }

            cumulative_ones += block_ones;
            cumulative_bits += BASIC_BLOCK_SIZE;
        }

        Self {
            buf,
            rank1_samples_pow2,
            select0_samples_pow2: select_samples_pow2,
            select1_samples_pow2: select_samples_pow2,
            rank1_samples: rank1_samples.into(),
            select0_samples: select0_samples.into(),
            select1_samples: select1_samples.into(),
            basic_blocks_per_rank1_sample_pow2: rank1_samples_pow2 - BASIC_BLOCK_BITS,
        }
    }

    fn rank1(&self, bit_index: u32) -> u32 {
        todo!()
    }

    fn rank0(&self, bit_index: u32) -> u32 {
        todo!()
    }

    fn select1(&self, n: u32) -> Option<u32> {
        todo!()
    }

    fn select0(&self, n: u32) -> Option<u32> {
        todo!()
    }

    fn has_rank0(&self) -> bool {
        todo!()
    }

    fn num_ones(&self) -> u32 {
        todo!()
    }
    fn num_zeros(&self) -> u32 {
        todo!()
    }
    fn universe_size(&self) -> u32 {
        todo!()
    }
    fn has_multiplicity(&self) -> bool {
        todo!()
    }
    fn num_unique_zeros(&self) -> u32 {
        todo!()
    }
    fn num_unique_ones(&self) -> u32 {
        todo!()
    }
}
