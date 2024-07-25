use crate::bits::partition_point;

pub trait BitVec: Clone {
    fn num_ones(&self) -> u32;
    fn num_zeros(&self) -> u32;
    // todo: make a decision about whether to allow a universe size of 2^32
    // (see comment in types.d.ts)
    fn universe_size(&self) -> u32;
    fn has_multiplicity(&self) -> bool;
    fn num_unique_zeros(&self) -> u32;
    fn num_unique_ones(&self) -> u32;

    /// Get the value of the bit at the specified index (0 or 1).
    /// Note: This is rather inefficient since it does two rank calls,
    /// each of which takes O(log(n)) time.
    ///
    /// In the presence of multiplicity, returns the count of the bit.
    fn get(&self, bit_index: u32) -> u32 {
        assert!(bit_index < self.universe_size());
        self.rank1(bit_index + 1) - self.rank1(bit_index)
    }

    // Returns the number of ones below the given bit index.
    fn rank1(&self, bit_index: u32) -> u32;

    fn rank0(&self, bit_index: u32) -> u32 {
        // The implementation below assumes no multiplicity;
        // otherwise, subtracting rank1 from the bit index can go negative.
        assert!(!self.has_multiplicity());
        if bit_index >= self.universe_size() {
            self.num_zeros()
        } else {
            bit_index - self.rank1(bit_index)
        }
    }

    fn select1(&self, n: u32) -> Option<u32> {
        if n >= self.num_ones() {
            return None;
        }
        // Binary search over rank1 to determine the position of the n-th 0-bit.
        let universe = self.universe_size() as usize;
        let bit_index = partition_point(universe, |i| self.rank1(i as u32) <= n) - 1;
        Some(bit_index as u32)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        if n >= self.num_zeros() {
            return None;
        }
        // Binary search over rank0 to determine the position of the n-th 0-bit.
        let universe = self.universe_size() as usize;
        let bit_index = partition_point(universe, |i| self.rank0(i as u32) <= n) - 1;
        Some(bit_index as u32)
    }

    /// Some `BitVec`s with multiplicity disallow 0-based queries because
    /// the representation does not support them. Multiplicity is a dynamic
    /// property so we use instance methods.
    fn has_rank0(&self) -> bool {
        true
    }

    fn has_select0(&self) -> bool {
        true
    }
}

pub trait BitVecBuilder {
    type Target: BitVec;
    fn new(universe_size: u32) -> Self;
    fn one_count(&mut self, bit_index: u32, count: u32);
    fn one(&mut self, bit_index: u32) {
        self.one_count(bit_index, 1);
    }
    fn build(self) -> Self::Target;
}
