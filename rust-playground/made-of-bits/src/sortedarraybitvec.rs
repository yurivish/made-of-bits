struct SortedArrayBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
}

impl SortedArrayBitVecBuilder {
    fn new(universe_size: u32) -> Self {
        Self {
            universe_size,
            ones: Vec::new(),
        }
    }

    fn one_count(&mut self, index: u32, count: u32) {
        assert!(index < self.universe_size);
        for _ in 0..count {
            self.ones.push(index);
        }
    }

    fn one(&mut self, index: u32) {
        self.one_count(index, 1)
    }

    fn build(mut self) -> SortedArrayBitVec {
        self.ones.sort();
        SortedArrayBitVec::new(self.ones.into(), self.universe_size)
    }
}

struct SortedArrayBitVec {
    ones: Box<[u32]>,
    universe_size: u32,
    num_ones: u32,
    num_zeros: u32,
    has_multiplicity: bool,
    num_unique_ones: u32,
    num_unique_zeros: u32,
}

impl SortedArrayBitVec {
    fn new(ones: Box<[u32]>, universe_size: u32) -> Self {
        let mut num_unique_ones = 0;
        let mut has_multiplicity = false;
        let mut prev = None;
        for (i, cur) in ones.iter().copied().enumerate() {
            let same = prev == Some(cur);
            has_multiplicity |= same;
            num_unique_ones += same as u32;
            if let Some(prev) = prev {
                debug_assert!(prev <= cur, "ones must be sorted")
            }
            prev = Some(cur);
        }

        let num_ones = ones.len() as u32;
        // Zeros are never repeated, so any non-one bits are singleton zeros.
        let num_zeros = universe_size - num_unique_ones;

        Self {
            ones,
            universe_size,
            num_ones,
            num_zeros,
            has_multiplicity: has_multiplicity,
            num_unique_ones,
            num_unique_zeros: num_zeros,
        }
    }

    fn rank1(&self, index: u32) -> u32 {
        self.ones.partition_point(|x| *x < index) as u32
    }

    // todo: default rank0 from trait

    fn select1(&self, n: u32) -> Option<u32> {
        Some(self.ones[n as usize])
    }

    // todo: default
    fn select0(&self, n: u32) -> Option<u32> {
        // Some(self.ones[n])
        todo!()
    }

    // todo: default get
    fn get(&self, index: u32) -> bool {
        let value = self.rank1(index + 1) - self.rank1(index);
        value != 0
    }
}
