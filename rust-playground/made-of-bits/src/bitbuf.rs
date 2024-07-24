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
        let spec = PadSpec::new(&mut self);
        PaddedBitBuf::new(self, spec)
    }
}

/// Represents a recommended padding for a particular BitVec.
/// We store this as its own struct so that we can use this
/// information to decide whether to compress a BitVec into
/// a PaddedBitVec based on a user-defined target compression
/// ratio, and also to do the compression. (Computing a PadSpec
/// requires a scan over the blocks).
#[derive(Default, Clone)]
struct PadSpec {
    padding: u32,
    padded_range: Range<usize>,
}

/// Compute the range of `arr` that contains no padding on its left
/// or right, analogous to a string trim operation, but returning the
/// index range rather than a slice.
fn padded_range(arr: &[u32], padding: u32) -> Range<usize> {
    let Some(start) = arr.iter().position(|&x| x != padding) else {
        // Return the empty range if the entire arrange consists of padding
        return 0..0;
    };
    // Slicing `arr` allows us to do at most a single full scan over the blocks
    let end = start + arr[start..].iter().rposition(|&x| x != padding).unwrap() + 1;
    start..end
}

struct PaddedBitBuf {
    blocks: Box<[u32]>,
    padding: u32,

    /// Index of the first non-padding block
    left_block_offset: u32,

    /// One beyond the last non-padding block
    right_block_offset: u32,

    /// Universe size of the original BitBuf
    universe_size: u32,

    /// Number of trailing bits in the original BitBuf
    num_trailing_bits: u32,
}

impl PadSpec {
    /// Note: Requires a mut reference because of a temporary modification to the last block.
    fn new(buf: &mut BitBuf) -> PadSpec {
        let zero_padding = 0; // a block of zeros
        let zero_padded_range = padded_range(&buf.blocks, zero_padding);

        // While counting 1-padding, temporarily set the highest `num_trailing_bits`
        // of the last block to 1, since otherwise we would wrongly not compress that block.
        let trailing_mask = !one_mask(BASIC_BLOCK_SIZE - buf.num_trailing_bits);
        let Some(last_block) = buf.blocks.last().copied() else {
            return Default::default();
        };
        let one_padding = u32::MAX; // a block of ones
        buf.blocks[buf.blocks.len() - 1] |= trailing_mask;
        let one_padded_range = padded_range(&buf.blocks, one_padding);
        // Reset the last block to its original state
        buf.blocks[buf.blocks.len() - 1] = last_block;

        // pick the padding that results in the shorter blocks array, or zero in case of a tie.
        if zero_padded_range.len() <= one_padded_range.len() {
            PadSpec {
                padding: zero_padding,
                padded_range: zero_padded_range,
            }
        } else {
            PadSpec {
                padding: one_padding,
                padded_range: one_padded_range,
            }
        }
    }
}

impl PaddedBitBuf {
    fn new(buf: BitBuf, spec: PadSpec) -> Self {
        let PadSpec {
            padded_range,
            padding,
        } = spec;
        let left_block_offset = padded_range.start as u32;
        let right_block_offset = padded_range.end as u32;
        let blocks = if padded_range.len() == buf.blocks.len() {
            buf.blocks
        } else {
            buf.blocks[padded_range].to_vec().into_boxed_slice()
        };

        Self {
            blocks,
            left_block_offset,
            right_block_offset,
            padding,
            universe_size: buf.universe_size,
            num_trailing_bits: buf.num_trailing_bits,
        }
    }

    /// Try padding the blocks of `buf` with zeros and ones, and return a `PadSpec`
    /// containing the best padding type as well as the padded range.

    fn should_pad(buf: &BitBuf, spec: PadSpec, compression_threshold: f64) -> bool {
        let num_blocks = buf.num_blocks();
        let num_compressed_blocks = spec.padded_range.len();
        num_compressed_blocks as f64 / num_blocks as f64 <= compression_threshold
    }

    fn get(&self, bit_index: u32) -> bool {
        let block_index = basic_block_index(bit_index);
        let bit = 1 << basic_block_offset(bit_index);
        let block =
            if block_index < self.left_block_offset || block_index >= self.right_block_offset {
                self.padding
            } else {
                self.blocks[(block_index - self.left_block_offset) as usize]
            };
        block & bit != 0
    }

    fn get_block(&self, block_index: u32) -> u32 {
        if block_index < self.left_block_offset || block_index >= self.right_block_offset {
            self.padding
        } else {
            self.blocks[(block_index - self.left_block_offset) as usize]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    /// Run a number of checks on `buf` and a PaddedBuf
    /// constructed from it after each modification.
    fn check(mut buf: BitBuf, offset: u32) {
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

        buf.set_one(offset + 2);
        assert_eq!(buf.get(offset + 0), false);
        assert_eq!(buf.get(offset + 1), true);
        assert_eq!(buf.get(offset + 2), true);

        {
            let buf = buf.clone().into_padded();
            assert_eq!(buf.get(offset + 0), false);
            assert_eq!(buf.get(offset + 1), true);
            assert_eq!(buf.get(offset + 2), true);
        }

        buf.set_one(offset + 0);
        assert_eq!(buf.get(offset + 0), true);
        assert_eq!(buf.get(offset + 1), true);
        assert_eq!(buf.get(offset + 2), true);

        {
            let buf = buf.clone().into_padded();
            assert_eq!(buf.get(offset + 0), true);
            assert_eq!(buf.get(offset + 1), true);
            assert_eq!(buf.get(offset + 2), true);
        }

        buf.set_zero(offset + 1);
        assert_eq!(buf.get(offset + 0), true);
        assert_eq!(buf.get(offset + 1), false);
        assert_eq!(buf.get(offset + 2), true);

        buf.set_zero(offset + 2);
        assert_eq!(buf.get(offset + 0), true);
        assert_eq!(buf.get(offset + 1), false);
        assert_eq!(buf.get(offset + 2), false);
        {
            let buf = buf.clone().into_padded();
            assert_eq!(buf.get(offset + 0), true);
            assert_eq!(buf.get(offset + 1), false);
            assert_eq!(buf.get(offset + 2), false);
        }

        buf.set_zero(offset + 0);
        assert_eq!(buf.get(offset + 0), false);
        assert_eq!(buf.get(offset + 1), false);
        assert_eq!(buf.get(offset + 2), false);
        {
            let buf = buf.clone().into_padded();
            assert_eq!(buf.get(offset + 0), false);
            assert_eq!(buf.get(offset + 1), false);
            assert_eq!(buf.get(offset + 2), false);
        }

        // should correctly report its number of blocks
        assert_eq!(buf.num_blocks(), buf.blocks.len() as u32);

        // should panic if manipulating out-of-bounds
        let mut buf_clone = buf.clone();
        assert!(
            panic::catch_unwind(move || { buf_clone.set_one(buf_clone.universe_size) }).is_err()
        );
        let mut buf_clone = buf.clone();
        assert!(
            panic::catch_unwind(move || { buf_clone.set_zero(buf_clone.universe_size) }).is_err()
        );
        let mut buf_clone = buf.clone();
        assert!(panic::catch_unwind(move || { buf_clone.get(buf_clone.universe_size) }).is_err());
        let mut buf_clone = buf.clone();
        assert!(
            panic::catch_unwind(move || { buf_clone.set_one(buf_clone.universe_size) }).is_err()
        );
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

    #[test]
    fn test_padded_bitbuf() {
        // should handle zero-width bufs
        assert!(panic::catch_unwind(move || { BitBuf::new(0).get(0) }).is_err());
        assert!(panic::catch_unwind(move || { BitBuf::new(0).get_block(0) }).is_err());

        // empty BitBufs should turn into blockless padded arrays
        assert_eq!(BitBuf::new(3).into_padded().blocks.len(), 0);
        assert_eq!(BitBuf::new(5).into_padded().blocks.len(), 0);
        assert_eq!(BitBuf::new(300).into_padded().blocks.len(), 0);

        {
            // should zero-pad to the leftmost and rightmost one
            let mut buf = BitBuf::new(123456);
            buf.set_one(0 * 32_000);
            buf.set_one(32_000 / 2);
            buf.set_one(1 * 32_000 - 1);

            // should return the correct suggestion for whether to pad or not
            // based on the provided compression ratio
            let spec = PadSpec::new(&mut buf);
            assert!(PaddedBitBuf::should_pad(&buf, spec.clone(), 1.0));
            assert!(PaddedBitBuf::should_pad(&buf, spec.clone(), 0.5));
            assert!(!PaddedBitBuf::should_pad(&buf, spec.clone(), 0.1));
            assert!(!PaddedBitBuf::should_pad(&buf, spec.clone(), 0.0));

            {
                // a zero-padded buffer is returned
                let buf = buf.clone().into_padded();
                assert_eq!(buf.blocks.len(), 1000);
                assert_eq!(buf.get(1), false);
                assert_eq!(buf.get(12345), false);
            }
        }

        {
            // should one-pad to the leftmost and rightmost one
            let mut buf = BitBuf::new(123456);
            buf.blocks.fill(u32::MAX);
            buf.set_zero(0 * 32_000);
            buf.set_zero(32_000 / 2);
            buf.set_zero(1 * 32_000 - 1);

            // should return the correct suggestion for whether to pad or not
            // based on the provided compression ratio
            let spec = PadSpec::new(&mut buf);
            assert!(PaddedBitBuf::should_pad(&buf, spec.clone(), 1.0));
            assert!(PaddedBitBuf::should_pad(&buf, spec.clone(), 0.5));
            assert!(!PaddedBitBuf::should_pad(&buf, spec.clone(), 0.1));
            assert!(!PaddedBitBuf::should_pad(&buf, spec.clone(), 0.0));

            {
                // a one-padded buffer is returned
                let buf = buf.clone().into_padded();
                assert_eq!(buf.blocks.len(), 1000);
                assert_eq!(buf.get(1), true);
                assert_eq!(buf.get(12345), true);
            }
        }

        {
            // should one-pad even with trailing bits in the last block
            let mut buf = BitBuf::new(50);
            for i in 0..50 {
                buf.set_one(i)
            }
            let buf = buf.into_padded();
            assert!(buf.blocks.is_empty());
            assert!(buf.padding == u32::MAX);
        }
    }
}
