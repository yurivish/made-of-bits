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
    fn get(&self, index: u32) -> u32 {
        assert!(index < self.universe_size());
        self.rank1(index + 1) - self.rank1(index)
    }

    fn rank1(&self, index: u32) -> u32;

    fn rank0(&self, index: u32) -> u32 {
        assert!(!self.has_multiplicity());
        if index == 0 {
            0
        } else if index >= self.universe_size() {
            self.num_zeros()
        } else {
            index - self.rank1(index)
        }
    }

    fn select1(&self, n: u32) -> Option<u32> {
        if n >= self.num_zeros() {
            return None;
        }
        // Binary search over rank0 to determine the position of the n-th 0-bit.
        let universe = self.universe_size() as usize;
        let index = partition_point(universe, |i| self.rank1(i as u32) <= n) as u32;
        Some(index - 1)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        if n >= self.num_zeros() {
            return None;
        }
        // Binary search over rank0 to determine the position of the n-th 0-bit.
        let universe = self.universe_size() as usize;
        let index = partition_point(universe, |i| self.rank0(i as u32) <= n) as u32;
        Some(index - 1)
    }

    /// Some BitVec types with multiplicity disallow 0-based queries because
    /// the representation does not support it. Maybe there's a better way
    /// to express this with traits, but for now we add a boolean flag so
    /// we can check this condition during testing.
    fn allows_rank0_and_select0() -> bool {
        return true;
    }
}

pub trait BitVecBuilder {
    type Target: BitVec;
    fn new(universe_size: u32) -> Self;
    fn one_count(&mut self, index: u32, count: u32);
    fn one(&mut self, index: u32) {
        self.one_count(index, 1);
    }
    fn build(self) -> Self::Target;
}
