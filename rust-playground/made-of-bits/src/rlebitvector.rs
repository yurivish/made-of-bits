use crate::{
    bits::partition_point,
    bitvec::{BitVec, BitVecBuilder},
    sparsebitvec::SparseBitVec,
};

// todo
// - consider implementing aligned rank0 and rank1, with debug asserts
/*
pub fn aligned_rank0(&self, index: Ones) -> Ones {
    if index >= self.len {
        return self.num_zeros;
    };

    // Number of complete 01-runs up to virtual index i
    let j = self.zo.rank1(index);

    // Number of zeros preceding the (aligned) index i
    self.z.select1(j + Ones::one())
}

pub fn aligned_rank1(&self, index: Ones) -> Ones {
    if index >= self.len {
        return self.num_ones;
    };
    index - self.aligned_rank0(index) + Ones::one()
}
 */

pub struct RLEBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
}

impl BitVecBuilder for RLEBitVecBuilder {
    type Target = RLEBitVec;

    fn new(universe_size: u32) -> Self {
        Self {
            universe_size,
            ones: Vec::new(),
        }
    }

    fn one_count(&mut self, bit_index: u32, count: u32) {
        assert!(bit_index < self.universe_size);
        for _ in 0..count {
            self.ones.push(bit_index);
        }
    }

    fn build(mut self) -> RLEBitVec {
        self.ones.sort();
        let mut builder = RLERunBuilder::new();
        let mut prev = u32::MAX;
        for &cur in &self.ones {
            let num_preceding_zeros = cur.wrapping_sub(prev) - 1;
            builder.run(num_preceding_zeros, 1);
            prev = cur;
        }
        // pad out with zeros if needed
        let num_zeros = self.universe_size.wrapping_sub(prev) - 1;
        builder.run(num_zeros, 0);
        builder.build()
    }
}

// Run-specific bitvector builder. Does not implement the BitVecBuilder interface.
struct RLERunBuilder {
    z: Vec<u32>,
    zo: Vec<u32>,
    num_zeros: u32,
    num_ones: u32,
}

impl RLERunBuilder {
    fn new() -> Self {
        Self {
            z: Vec::new(),
            zo: Vec::new(),
            num_zeros: 0,
            num_ones: 0,
        }
    }

    fn run(&mut self, num_zeros: u32, num_ones: u32) {
        if num_zeros == 0 && num_ones == 0 {
            return;
        }
        let len = self.z.len();
        self.num_zeros += num_zeros;
        self.num_ones += num_ones;
        if num_zeros == 0 && len > 0 {
            // this run consists of only ones; coalesce it with the
            // previous run (since all runs contain ones at their end).
            *self.zo.last_mut().unwrap() += num_ones;
        } else if num_ones == 0 && self.last_block_contains_only_zeros() {
            // this run consists of only zeros; coalesce it with the
            // previous run (since it turns out to consist of only zeros).
            *self.z.last_mut().unwrap() += num_zeros;
            *self.zo.last_mut().unwrap() += num_zeros;
        } else {
            // No coalescing is possible; create a new block of runs.
            // Append the cumulative number of zeros to the Z array
            self.z.push(self.num_zeros);
            // Append the cumulative number of ones and zeros to the ZO array
            self.zo.push(self.num_zeros + self.num_ones);
        }
    }

    fn last_block_contains_only_zeros(&self) -> bool {
        let len = self.z.len();
        match len {
            0 => false,
            1 => self.z[0] == self.zo[0],
            _ => {
                let last_block_length = self.zo[len - 1] - self.zo[len - 2];
                let last_block_num_zeros = self.z[len - 1] - self.z[len - 2];
                last_block_length == last_block_num_zeros
            }
        }
    }

    fn build(self) -> RLEBitVec {
        // The +1 to the universe size is needed because the 1-bit marker in z
        // comes at the position after `self.num_zeros` zeros, and the same idea
        // applies to zo, which marks with a 1-bit the position after each 01-run.
        RLEBitVec {
            z: SparseBitVec::new(self.z.into(), self.num_zeros + 1),
            zo: SparseBitVec::new(self.zo.into(), self.num_zeros + self.num_ones + 1),
            num_zeros: self.num_zeros,
            num_ones: self.num_ones,
        }
    }
}

#[derive(Clone)]
pub struct RLEBitVec {
    z: SparseBitVec,
    zo: SparseBitVec,
    num_zeros: u32,
    num_ones: u32,
}

impl BitVec for RLEBitVec {
    fn rank1(&self, bit_index: u32) -> u32 {
        if bit_index >= self.universe_size() {
            return self.num_ones;
        }

        // Number of complete 01-runs up to the virtual index `bit_index`
        let j = self.zo.rank1(bit_index);

        // Number of zeros including the j-th block
        let num_cumulative_zeros = self.z.select1(j).unwrap();

        // Note: Below, wrapping_sub relies on the fact that bit_index
        // cannot be u32::MAX since the universe size is representable
        // as u32, implying a maximum index of u32::MAX - 1.

        // Number of zeros preceding the j-th block
        let num_preceding_zeros = self.z.select1(j.wrapping_sub(1)).unwrap_or(0);

        // Number of zeros in the j-th block
        let num_zeros = num_cumulative_zeros - num_preceding_zeros;

        // Start index of the j-th block
        let block_start = self.zo.select1(j.wrapping_sub(1)).unwrap_or(0);

        // Number of ones preceding the j-th block
        let num_preceding_ones = block_start - num_preceding_zeros;

        // Start index of ones in the j-th block
        let ones_start = block_start + num_zeros;

        let adjustment = bit_index.saturating_sub(ones_start);

        num_preceding_ones + adjustment
    }

    fn select1(&self, n: u32) -> Option<u32> {
        if n >= self.num_ones {
            return None;
        }

        // The n-th one is in the j-th 01-block.
        let j = partition_point(self.z.num_ones() as usize, |i| {
            let i = i as u32;
            self.zo.select1(i).unwrap() - self.z.select1(i).unwrap() <= n
        }) as u32;

        // Number of zeros up to and including the j-th block
        let num_cumulative_zeros = self.z.select1(j).unwrap();

        Some(num_cumulative_zeros + n)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        if n >= self.num_zeros {
            return None;
        };

        // The n-th zero is in the j-th 01-block.
        let j = self.z.rank1(n + 1);

        // If we're in the first 01-block, the n-th zero is at index n.
        if j == 0 {
            return Some(n);
        };

        // Start index of the j-th 01-block
        let block_start = self.zo.select1(j - 1).unwrap();

        // Number of zeros preceding the j-th 01-block
        let num_preceding_zeros = self.z.select1(j - 1).unwrap();

        // Return the index of the (n - numPrecedingZeros)-th zero in the j-th 01-block.
        Some(block_start + (n - num_preceding_zeros))
    }

    fn num_ones(&self) -> u32 {
        self.num_ones
    }

    fn universe_size(&self) -> u32 {
        self.num_zeros + self.num_ones
    }

    fn num_unique_zeros(&self) -> u32 {
        self.num_zeros
    }

    fn num_unique_ones(&self) -> u32 {
        self.num_ones
    }

    fn supports_multiplicity() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::test_bitvec::test_bit_vec_builder;
    use crate::test_bitvec::test_bit_vec_builder_arbtest;

    use super::*;

    #[test]
    fn test() {
        test_bit_vec_builder::<RLEBitVecBuilder>();
        // test_bit_vec_builder_arbtest::<RLEBitVecBuilder>(None, None, false);
    }
}
