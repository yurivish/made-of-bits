use crate::{
    bits::partition_point,
    bitvec::{BitVec, BitVecBuilder, BitVecOf},
    bitvecs::sparse::SparseBitVec,
};
use std::collections::HashSet;

#[derive(Clone)]
pub struct RLEBitVec {
    /// z[i]: cumulative number of zeros before the start of the i-th 1-run;
    /// can be thought of as pointing to the index of the first 1 in a 01-run.
    /// Since we coalesce runs there are no zero-length runs, and therefore we
    /// can use a bitvector type without multiplicity here and for `zo` (though
    /// this isn't strictly required and is done for clarity and to enforce the
    /// invariant with the quick runtime check in `BitVecOf` construction).
    z: BitVecOf<SparseBitVec>,
    /// zo[i]: cumulative number of ones and zeros at the end of the i-th 01-run;
    /// can be thought of as pointing just beyond the index of the last 1 in a 01-run.
    zo: BitVecOf<SparseBitVec>,
    num_zeros: u32,
    num_ones: u32,
}

impl RLEBitVec {
    // TODO: Document (and debug_assert) the invariants of these functions.
    // They are more efficient versions of rank0 and rank1 that take advantage of the fact
    // that the queries happen precisely on run boundaries.
    pub fn aligned_rank0(&self, bit_index: u32) -> u32 {
        if bit_index >= self.universe_size() {
            return self.num_zeros;
        };

        // Number of complete 01-runs up to virtual index i
        let j = self.zo.rank1(bit_index);

        // Number of zeros preceding the (aligned) index i
        self.z.select1(j + 1).unwrap()
    }

    pub fn aligned_rank1(&self, bit_index: u32) -> u32 {
        if bit_index >= self.universe_size() {
            return self.num_ones;
        };
        bit_index - self.aligned_rank0(bit_index) + 1
    }
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

    fn universe_size(&self) -> u32 {
        self.num_zeros + self.num_ones
    }

    fn num_ones(&self) -> u32 {
        self.num_ones
    }

    fn num_zeros(&self) -> u32 {
        self.num_zeros
    }
}

pub struct RLEBitVecBuilder {
    universe_size: u32,
    ones: HashSet<u32>,
}

impl BitVecBuilder for RLEBitVecBuilder {
    type Target = RLEBitVec;

    fn new(universe_size: u32) -> Self {
        Self {
            universe_size,
            ones: HashSet::new(),
        }
    }

    fn one(&mut self, bit_index: u32) {
        assert!(bit_index < self.universe_size);
        self.ones.insert(bit_index);
    }

    fn build(self) -> RLEBitVec {
        let mut ones = self.ones.into_iter().collect::<Vec<_>>();
        ones.sort();
        let mut b = RLEBitVecRunBuilder::new();
        let mut prev = u32::MAX;
        for cur in ones {
            let num_preceding_zeros = cur.wrapping_sub(prev) - 1;
            b.run(num_preceding_zeros, 1);
            prev = cur;
        }
        // pad out with zeros if needed
        let num_zeros = self.universe_size.wrapping_sub(prev) - 1;
        b.run(num_zeros, 0);
        b.build()
    }
}

// Run-specific bitvector builder. Does not implement the BitVecBuilder interface.
struct RLEBitVecRunBuilder {
    z: Vec<u32>,
    zo: Vec<u32>,
    num_zeros: u32,
    num_ones: u32,
}

impl RLEBitVecRunBuilder {
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

    fn build(self) -> RLEBitVec {
        assert!(
            self.num_zeros + self.num_ones < u32::MAX - 1,
            "maximum allowed universe size is 2^32-2"
        );

        // The +1 to the universe size is needed because the 1-bit marker in z
        // comes at the position after `self.num_zeros` zeros, and the same idea
        // applies to zo, which marks with a 1-bit the position after each 01-run.
        RLEBitVec {
            z: BitVecOf::new(SparseBitVec::new(self.z.into(), self.num_zeros + 1)),
            zo: BitVecOf::new(SparseBitVec::new(
                self.zo.into(),
                self.num_zeros + self.num_ones + 1,
            )),
            num_zeros: self.num_zeros,
            num_ones: self.num_ones,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec_test::*;

    #[test]
    fn test() {
        test_bitvec_builder::<RLEBitVecBuilder>();
    }
}
