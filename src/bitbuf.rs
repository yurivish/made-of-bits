use crate::bitblock::BitBlock;
use crate::bits::one_mask;
use std::ops::Range;

/// Block type for BitBuf blocks. u64 matches the native machine word, letting
/// broadword `select64` operate at full register width. Block-level arithmetic
/// throughout the crate is width-agnostic via `Block::BITS`, `block_index`, and
/// `block_bit_index`.
pub(crate) type Block = u64;

#[derive(Clone)]
pub(crate) struct BitBuf {
    blocks: Box<[Block]>,
    universe_size: u32,
    num_trailing_bits: u32,
}

impl BitBuf {
    /// Construct a new `BitBuf` containing all 0-bits.
    /// `universe_size` is the length of this bit buffer in bits.
    pub(crate) fn new(universe_size: u32) -> Self {
        let num_blocks = universe_size.div_ceil(Block::BITS);
        let last_block_occupancy = universe_size % Block::BITS;
        let num_trailing_bits = if last_block_occupancy == 0 {
            0
        } else {
            Block::BITS - last_block_occupancy
        };
        Self {
            blocks: vec![0; num_blocks as usize].into(),
            universe_size,
            num_trailing_bits,
        }
    }

    /// Set the bit at index `bit_index` to a 1-bit.
    pub(crate) fn set_one(&mut self, bit_index: u32) {
        debug_assert!(bit_index < self.universe_size);
        let block_index = Block::block_index(bit_index);
        let bit = 1 << Block::block_bit_index(bit_index);
        self.blocks[block_index] |= bit
    }

    /// Set the bit at index `bit_index` to a 0-bit.
    pub(crate) fn set_zero(&mut self, bit_index: u32) {
        debug_assert!(bit_index < self.universe_size);
        let block_index = Block::block_index(bit_index);
        let bit = 1 << Block::block_bit_index(bit_index);
        self.blocks[block_index] &= !bit
    }

    pub(crate) fn get(&self, bit_index: u32) -> bool {
        debug_assert!(bit_index < self.universe_size);
        let block_index = Block::block_index(bit_index);
        let bit = 1 << Block::block_bit_index(bit_index);
        self.blocks[block_index] & bit != 0
    }

    pub(crate) fn block(&self, block_index: u32) -> Block {
        self.blocks[block_index as usize]
    }

    pub(crate) fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub(crate) fn num_blocks(&self) -> u32 {
        // The number of blocks fits in a u32 by construction
        // since it is no greater than the universe_size.
        self.blocks.len() as u32
    }

    pub(crate) fn num_trailing_bits(&self) -> u32 {
        self.num_trailing_bits
    }

    pub(crate) fn universe_size(&self) -> u32 {
        self.universe_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panics;

    /// Run a number of checks on `buf`
    /// constructed from it after each modification.
    fn check(mut buf: BitBuf, offset: u32) {
        // should be initialized to
        assert_eq!(buf.get(offset + 0), false);
        assert_eq!(buf.get(offset + 1), false);
        assert_eq!(buf.get(offset + 2), false);

        // should set and un-set bits individually
        buf.set_one(offset + 1);
        assert_eq!(buf.get(offset + 0), false);
        assert_eq!(buf.get(offset + 1), true);
        assert_eq!(buf.get(offset + 2), false);

        buf.set_one(offset + 2);
        assert_eq!(buf.get(offset + 0), false);
        assert_eq!(buf.get(offset + 1), true);
        assert_eq!(buf.get(offset + 2), true);

        buf.set_one(offset + 0);
        assert_eq!(buf.get(offset + 0), true);
        assert_eq!(buf.get(offset + 1), true);
        assert_eq!(buf.get(offset + 2), true);

        buf.set_zero(offset + 1);
        assert_eq!(buf.get(offset + 0), true);
        assert_eq!(buf.get(offset + 1), false);
        assert_eq!(buf.get(offset + 2), true);

        buf.set_zero(offset + 2);
        assert_eq!(buf.get(offset + 0), true);
        assert_eq!(buf.get(offset + 1), false);
        assert_eq!(buf.get(offset + 2), false);

        buf.set_zero(offset + 0);
        assert_eq!(buf.get(offset + 0), false);
        assert_eq!(buf.get(offset + 1), false);
        assert_eq!(buf.get(offset + 2), false);

        // should correctly report its number of blocks
        assert_eq!(buf.num_blocks(), buf.blocks.len() as u32);

        // should panic if manipulating out-of-bounds
        let mut buf_clone = buf.clone();
        assert!(panics(|| buf_clone.set_one(buf_clone.universe_size)));
        let mut buf_clone = buf.clone();
        assert!(panics(|| buf_clone.set_zero(buf_clone.universe_size)));
        let buf_clone = buf.clone();
        assert!(panics(|| buf_clone.get(buf_clone.universe_size)));
        let mut buf_clone = buf.clone();
        assert!(panics(|| buf_clone.set_one(buf_clone.universe_size)));
    }

    #[test]
    fn test_bitbuf() {
        // should handle zero-width bufs
        assert!(panics(|| BitBuf::new(0).set_one(0)));
        assert!(panics(|| BitBuf::new(0).set_zero(0)));
        assert!(panics(|| BitBuf::new(0).get(0)));

        // should handle max-size bufs
        BitBuf::new(u32::MAX);

        check(BitBuf::new(3), 0);
        check(BitBuf::new(5), 2);
        check(BitBuf::new(300), 0);
        check(BitBuf::new(300), 100);
    }

    /// Exercises bits at every u32 / u64 block boundary. Phase-1 block-widening
    /// regression net: previously u32 blocks made index 32 land in block 1; with
    /// u64 it lands in block 0. Either way, get must agree with set.
    #[test]
    fn block_boundary_bits() {
        let positions: &[u32] = &[
            0, 1, 30, 31, 32, 33, 62, 63, 64, 65, 94, 95, 96, 97, 126, 127, 128, 129,
        ];
        let universe = 200;
        let mut buf = BitBuf::new(universe);
        for &p in positions {
            buf.set_one(p);
        }
        for i in 0..universe {
            let expected = positions.contains(&i);
            assert_eq!(buf.get(i), expected, "get({i})");
        }
        // num_blocks reflects the new block width.
        assert_eq!(buf.num_blocks(), universe.div_ceil(Block::BITS));
    }
}
