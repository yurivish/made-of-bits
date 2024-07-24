use crate::bits::{basic_block_index, basic_block_offset, one_mask, BASIC_BLOCK_SIZE};
use std::ops::Range;

// todo: padded bitbuf and a trait across the two

#[derive(Clone)]
struct BitBuf {
    blocks: Box<[u32]>,
    universe_size: u32,
    num_trailing_bits: u32,
}

impl BitBuf {
    /// Construct a new `BitBuf` containing all 0-bits.
    /// `universe_size` is the length of this bit buffer in bits.
    fn new(universe_size: u32) -> Self {
        let num_blocks = universe_size.div_ceil(BASIC_BLOCK_SIZE);
        let last_block_occupancy = universe_size % BASIC_BLOCK_SIZE;
        let num_trailing_bits = if last_block_occupancy == 0 {
            0
        } else {
            BASIC_BLOCK_SIZE - last_block_occupancy
        };
        Self {
            blocks: vec![0; num_blocks as usize].into(),
            universe_size,
            num_trailing_bits,
        }
    }

    /// Set the bit at index `bit_index` to a 1-bit.
    fn set_one(&mut self, bit_index: u32) {
        debug_assert!(bit_index < self.universe_size);
        let block_index = basic_block_index(bit_index) as usize;
        let bit = 1 << basic_block_offset(bit_index);
        self.blocks[block_index] |= bit
    }

    /// Set the bit at index `bit_index` to a 0-bit.
    fn set_zero(&mut self, bit_index: u32) {
        debug_assert!(bit_index < self.universe_size);
        let block_index = basic_block_index(bit_index) as usize;
        let bit = 1 << basic_block_offset(bit_index);
        self.blocks[block_index] &= !bit
    }

    fn get(&self, bit_index: u32) -> bool {
        debug_assert!(bit_index < self.universe_size);
        let block_index = basic_block_index(bit_index) as usize;
        let bit = 1 << basic_block_offset(bit_index);
        self.blocks[block_index] & bit != 0
    }

    fn get_block(&self, block_index: u32) -> u32 {
        self.blocks[block_index as usize]
    }

    fn num_blocks(&self) -> u32 {
        // The number of blocks fits in a u32 by construction
        // since it is no greater than the universe_size.
        self.blocks.len() as u32
    }

    fn num_trailing_bits(&self) -> u32 {
        self.num_trailing_bits
    }

    fn universe_size(&self) -> u32 {
        self.universe_size
    }

    fn into_padded(mut self) -> PaddedBitBuf {
        let (padding, padded_range) = PaddedBitBuf::best_padding(&mut self);
        PaddedBitBuf::new(self, padding, padded_range)
    }
    // fn should_pad(&mut self, threshold: f64) -> Option<PaddedBitBuf> {
    //     let (block_padding, range) = PaddedBitBuf::best_padding(self);
    //     let num_blocks = self.num_blocks();
    //     let num_compressed_blocks = range.len();
    //     if (num_compressed_blocks as f64 / num_blocks as f64 <= threshold) {}
    //     // let padded = PaddedBitBuf::new(self)
    // }
}

fn padded_range(arr: &[u32], padding: u32) -> Range<usize> {
    // Compute the left and right indices of the blocks
    // we would like to keep, ie. which are non-padding.
    let start = arr.iter().position(|&x| x != padding).unwrap_or(arr.len());
    let end = arr.iter().rposition(|&x| x != padding).unwrap_or(0);
    start..end + 1
}

#[derive(Default)] // todo
struct PaddedBitBuf {
    blocks: Box<[u32]>,
    block_padding: u32,
    left_block_offset: u32,
    right_block_offset: u32,

    // These refer to the properties of the original BitBuf
    universe_size: u32,
    num_trailing_bits: u32,
}

impl PaddedBitBuf {
    // Try padding the blocks of `buf` with zeros and ones, and return the best padding, as
    // well as the padded range.
    // Note: Requires a mut reference because of a temporary modification to the last block.
    // Returns the block padding and the non-padded range.
    fn best_padding(buf: &mut BitBuf) -> (u32, Range<usize>) {
        let zero_padding = 0; // a block of zeros
        let zero_padded_range = padded_range(&buf.blocks, zero_padding);

        // While counting 1-padding, temporarily set the highest `num_trailing_bits`
        // of the last block to 1, since otherwise we would wrongly not compress that block.
        let trailing_mask = !one_mask(BASIC_BLOCK_SIZE - buf.num_trailing_bits);
        let Some(last_block) = buf.blocks.last().copied() else {
            return (0, 0..0);
        };
        let one_padding = u32::MAX; // a block of ones
        buf.blocks[buf.blocks.len() - 1] |= trailing_mask;
        let one_padded_range = padded_range(&buf.blocks, one_padding);
        // Reset the last block to its original state
        buf.blocks[buf.blocks.len() - 1] = last_block;

        // pick the padding that results in the shorter blocks array, or zero in case of a tie.
        if zero_padded_range.len() <= one_padded_range.len() {
            (zero_padding, zero_padded_range)
        } else {
            (one_padding, one_padded_range)
        }
    }

    fn new(buf: BitBuf, block_padding: u32, range: Range<usize>) -> Self {
        let left_block_offset = range.start as u32;
        let right_block_offset = range.end as u32;
        let blocks = if range.len() < buf.blocks.len() {
            buf.blocks[range].to_vec().into_boxed_slice()
        } else {
            buf.blocks
        };

        Self {
            blocks,
            left_block_offset,
            right_block_offset,
            block_padding,
            universe_size: buf.universe_size,
            num_trailing_bits: buf.num_trailing_bits,
        }
    }

    fn get(&self, bit_index: u32) -> bool {
        let block_index = basic_block_index(bit_index);
        let bit = 1 << basic_block_offset(bit_index);
        let block =
            if block_index < self.left_block_offset || block_index >= self.right_block_offset {
                self.block_padding
            } else {
                self.blocks[(block_index - self.left_block_offset) as usize]
            };
        block & bit != 0
    }

    fn get_block(&self, block_index: u32) -> u32 {
        if block_index < self.left_block_offset || block_index >= self.right_block_offset {
            self.block_padding
        } else {
            self.blocks[(block_index - self.left_block_offset) as usize]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    fn check(mut buf: BitBuf, offset: u32) {
        // Run a number of checks on `buf`, with the same
        // checks on a PaddedBuf constructed from it after
        // each mutation.

        // should be initialized to
        assert_eq!(buf.get(offset + 0), false);
        assert_eq!(buf.get(offset + 1), false);
        assert_eq!(buf.get(offset + 2), false);
        {
            let buf = buf.clone().into_padded();
            assert_eq!(buf.get(offset + 0), false);
            assert_eq!(buf.get(offset + 1), false);
            assert_eq!(buf.get(offset + 2), false);
        }

        // should set and un-set bits individually
        buf.set_one(offset + 1);
        assert_eq!(buf.get(offset + 0), false);
        assert_eq!(buf.get(offset + 1), true);
        assert_eq!(buf.get(offset + 2), false);
        {
            let buf = buf.clone().into_padded();
            assert_eq!(buf.get(offset + 0), false);
            assert_eq!(buf.get(offset + 1), true);
            assert_eq!(buf.get(offset + 2), false);
        }

        // buf.set_one(offset + 2);
        // assert_eq!(buf.get(offset + 0), false);
        // assert_eq!(buf.get(offset + 1), true);
        // assert_eq!(buf.get(offset + 2), true);

        // {
        //     let buf = buf.clone().into_padded();
        //     assert_eq!(buf.get(offset + 0), false);
        //     assert_eq!(buf.get(offset + 1), true);
        //     assert_eq!(buf.get(offset + 2), true);
        // }

        // buf.set_one(offset + 0);
        // assert_eq!(buf.get(offset + 0), true);
        // assert_eq!(buf.get(offset + 1), true);
        // assert_eq!(buf.get(offset + 2), true);

        // {
        //     let buf = buf.clone().into_padded();
        //     assert_eq!(buf.get(offset + 0), true);
        //     assert_eq!(buf.get(offset + 1), true);
        //     assert_eq!(buf.get(offset + 2), true);
        // }

        // buf.set_zero(offset + 1);
        // assert_eq!(buf.get(offset + 0), true);
        // assert_eq!(buf.get(offset + 1), false);
        // assert_eq!(buf.get(offset + 2), true);

        // buf.set_zero(offset + 2);
        // assert_eq!(buf.get(offset + 0), true);
        // assert_eq!(buf.get(offset + 1), false);
        // assert_eq!(buf.get(offset + 2), false);
        // {
        //     let buf = buf.clone().into_padded();
        //     assert_eq!(buf.get(offset + 0), true);
        //     assert_eq!(buf.get(offset + 1), false);
        //     assert_eq!(buf.get(offset + 2), false);
        // }

        // buf.set_zero(offset + 0);
        // assert_eq!(buf.get(offset + 0), false);
        // assert_eq!(buf.get(offset + 1), false);
        // assert_eq!(buf.get(offset + 2), false);
        // {
        //     let buf = buf.clone().into_padded();
        //     assert_eq!(buf.get(offset + 0), false);
        //     assert_eq!(buf.get(offset + 1), false);
        //     assert_eq!(buf.get(offset + 2), false);
        // }

        // // should correctly report its number of blocks
        // assert_eq!(buf.num_blocks(), buf.blocks.len() as u32);

        // // should panic if manipulating out-of-bounds
        // let mut buf_clone = buf.clone();
        // assert!(
        //     panic::catch_unwind(move || { buf_clone.set_one(buf_clone.universe_size) }).is_err()
        // );
        // let mut buf_clone = buf.clone();
        // assert!(
        //     panic::catch_unwind(move || { buf_clone.set_zero(buf_clone.universe_size) }).is_err()
        // );
        // let mut buf_clone = buf.clone();
        // assert!(panic::catch_unwind(move || { buf_clone.get(buf_clone.universe_size) }).is_err());
        // let mut buf_clone = buf.clone();
        // assert!(
        //     panic::catch_unwind(move || { buf_clone.set_one(buf_clone.universe_size) }).is_err()
        // );
    }

    #[test]
    fn test_bitbuf() {
        // should handle zero-width bufs
        assert!(panic::catch_unwind(move || { BitBuf::new(0).set_one(0) }).is_err());
        assert!(panic::catch_unwind(move || { BitBuf::new(0).set_zero(0) }).is_err());
        assert!(panic::catch_unwind(move || { BitBuf::new(0).get(0) }).is_err());

        check(BitBuf::new(3), 0);
        check(BitBuf::new(5), 2);
        check(BitBuf::new(300), 0);
        check(BitBuf::new(300), 100);
    }
}
