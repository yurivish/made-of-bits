use crate::bits::{basic_block_index, basic_block_offset, one_mask, BASIC_BLOCK_SIZE};

/// Fixed-size buffer of fixed-width integers. Designed to be written once and read many times.
/// A newly constructed IntBuf will have the specified length and all elements will be initialized to zero.
/// Elements can be added by pushing them onto the vector, which will add that element from the front at the lowest available index.
/// In typical use, the vector will be initialized and and then precisely `length` elements will be pushed.
pub struct IntBuf {
    blocks: Box<[u32]>,
    length: u32,
    length_in_bits: u32,
    bit_width: u32,
    low_bit_mask: u32,
    write_cursor: u32,
}

impl IntBuf {
    fn new(length: u32, bit_width: u32) -> Self {
        // The bit width cannot exceed BASIC_BLOCK_SIZE, since then a
        // single value could span more than two contiguous blocks and
        // our algorithms assume this cannot happen.
        assert!(bit_width <= BASIC_BLOCK_SIZE);

        let length_in_bits = length * bit_width;
        let num_blocks = length_in_bits.div_ceil(BASIC_BLOCK_SIZE);
        Self {
            blocks: vec![0; num_blocks as usize].into(),
            length,
            length_in_bits,
            bit_width,
            low_bit_mask: one_mask(bit_width),
            write_cursor: 0, // in bits
        }
    }

    /// Push a value into the IntBuf.
    /// Will panic if there is no room to store the value.
    /// Note that as a special case, this means that any number of
    /// zeros can be pushed to a IntBuf with bitWidth zero.
    fn push(&mut self, value: u32) {
        assert!(value < 1 << self.bit_width);
        // If we have zero bit width, only allow writing zeros (and there's no need to write them!)
        if self.bit_width == 0 {
            assert!(value == 0);
            return;
        }
        let index = basic_block_index(self.write_cursor);
        let offset = basic_block_offset(self.write_cursor);
        self.blocks[index] |= value << offset;

        // Number of bits available in the current block
        let num_available_bits = BASIC_BLOCK_SIZE - offset;

        // If needed, write any remaining bits into the next block.
        if num_available_bits < self.bit_width {
            self.blocks[index + 1] = value >> num_available_bits;
        }
        self.write_cursor += self.bit_width;
    }

    fn get(&self, index: u32) -> u32 {
        assert!(index < self.length);

        // If the bit width is zero, our vector is entirely full of zeros.
        if self.bit_width == 0 {
            return 0;
        }

        let bit_index = index * self.bit_width;
        let block_index = basic_block_index(bit_index);
        let offset = basic_block_offset(bit_index);

        let mut value = (self.blocks[block_index] & (self.low_bit_mask << offset)) >> offset;

        // Number of bits available in the current block
        let num_available_bits = BASIC_BLOCK_SIZE - offset;

        // If needed, extract the remaining bits from the bottom of the next block
        if num_available_bits < self.bit_width {
            let num_remaining_bits = self.bit_width - num_available_bits;
            let high_bits = self.blocks[block_index + 1];
            value |= high_bits << num_available_bits;
        }

        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intbuf() {
        //
    }
}
