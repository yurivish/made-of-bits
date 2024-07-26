use crate::bitbuf::{BitBuf, PaddedBitBuf};

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
    /// buf: bit buffer containing the underlying bit data
    /// rank1SamplesPow2: power of 2 of the rank1 sample rate
    /// selectSamplesPow2: power of 2 of the select sample rate for both select0 and select1
    fn new(buf: BitBuf, rank1_samples_pow2: u32, select_samples_pow2: u32) -> Self {
        todo!()
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
