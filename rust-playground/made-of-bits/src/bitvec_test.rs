// todo: do a clarity pass on these tests - they're somewhat hard to follow
use crate::{
    bits::BASIC_BLOCK_SIZE,
    bitvec::{BitVec, BitVecBuilder},
};
use std::{
    collections::BTreeMap,
    panic::{catch_unwind, UnwindSafe},
};

// 🌶️
// todo: port basic property tests with fisher yates shuffle for regular and multi bitvecs

pub(crate) fn test_bit_vec_builder<T: BitVecBuilder>()
where
    T::Target: UnwindSafe,
{
    // test the empty bitvec
    test_bit_vec(T::new(0).build());

    // large enough to span many blocks
    let universe_size = BASIC_BLOCK_SIZE * 10;
    {
        // save time by only testing with every `step`-th bit set
        let step = ((BASIC_BLOCK_SIZE >> 1) - 1);

        // test with one bit set
        for bit_index in (0..universe_size).step_by(step as usize) {
            let mut builder = T::new(universe_size);
            builder.one(bit_index);
            let bv = builder.build();
            test_bit_vec(bv.clone());

            assert_eq!(bv.rank1(bit_index), 0);
            assert_eq!(bv.rank1(bit_index + 1), 1);
            assert_eq!(bv.rank1(1_000_000), 1);

            assert_eq!(bv.rank0(bit_index), bit_index);
            assert_eq!(bv.rank0(bit_index + 1), bit_index);
            assert_eq!(bv.rank0(1_000_000), bv.universe_size() - 1);

            // select0
            if bit_index == 0 {
                assert_eq!(bv.select0(0), Some(1));
            } else {
                assert_eq!(bv.select0(0), Some(0));
                assert_eq!(bv.select0(bit_index - 1), Some(bit_index - 1));
            }

            if bit_index == bv.universe_size() - 1 {
                // if we're at the final index, there is no corresponding 0- or 1-bit
                assert_eq!(bv.select0(bit_index), None);
                assert_eq!(bv.select1(bit_index), None);
            } else {
                assert_eq!(bv.select0(bit_index), Some(bit_index + 1));
            }

            // select1
            assert_eq!(bv.select1(0), Some(bit_index));
            assert_eq!(bv.select1(1), None);
        }

        for bit_index_1 in (0..universe_size).step_by(step as usize) {
            for bit_index_2 in (bit_index_1 + step..universe_size).step_by(step as usize) {
                let mut builder = T::new(universe_size);
                builder.one(bit_index_1);
                builder.one(bit_index_2);
                let bv = builder.build();
                test_bit_vec(bv.clone());

                assert_eq!(bv.rank1(bit_index_1), 0);
                assert_eq!(bv.rank1(bit_index_1 + 1), 1);
                assert_eq!(bv.rank1(bit_index_2), 1);
                assert_eq!(bv.rank1(bit_index_2 + 1), 2);
                assert_eq!(bv.rank1(1_000_000), 2);

                assert_eq!(bv.rank0(bit_index_1), bit_index_1);
                assert_eq!(bv.rank0(bit_index_1 + 1), bit_index_1);
                assert_eq!(bv.rank0(bit_index_2), bit_index_2 - 1);
                assert_eq!(bv.rank0(bit_index_2 + 1), bit_index_2 - 1);
                assert_eq!(bv.rank0(1_000_000), bv.universe_size() - 2);

                // select0
                // with 2 bits the edge cases are complex to express, so just test the first element
                assert_eq!(
                    bv.select0(0),
                    Some((bit_index_1 == 0) as u32 + (bit_index_1 == 0 && bit_index_2 == 1) as u32)
                );

                // select1
                assert_eq!(bv.select1(0), Some(bit_index_1));
                assert_eq!(bv.select1(1), Some(bit_index_2));
                assert_eq!(bv.select1(2), None);
            }
        }
    }
}

pub(crate) fn test_bit_vec<T: BitVec + UnwindSafe>(bv: T) {
    assert!(bv.num_unique_zeros() + bv.num_unique_ones() == bv.universe_size());
    assert!(bv.num_zeros() + bv.num_ones() >= bv.universe_size());

    // Rank before any element should be zero
    assert_eq!(bv.rank1(0), 0);
    assert_eq!(bv.rank1(bv.num_zeros() + bv.num_ones() + 1), bv.num_ones());

    if bv.has_rank0() {
        assert_eq!(bv.rank0(0), 0);
        assert_eq!(bv.rank0(bv.num_zeros() + bv.num_ones() + 1), bv.num_zeros());
    }

    // select1
    for n in 0..bv.num_ones() {
        // Verify that rank1(select1(n)) === n
        let select1 = bv.select1(n).unwrap();
        assert!(bv.rank1(select1) == n);
        assert!(bv.rank1(select1 + 1) == n + 1);
    }

    if bv.has_rank0() && bv.has_select0() {
        // select0
        for n in 0..bv.num_zeros() {
            // Verify that rank0(select0(n)) === n
            let select0 = bv.select0(n).unwrap();
            assert!(bv.rank0(select0) == n);
            assert!(bv.rank0(select0 + 1) == n + 1);
        }
    }

    if !bv.has_multiplicity() {
        // Perform more stringent checks when we know that multiplicity is not in play
        assert!(bv.num_zeros() + bv.num_ones() == bv.universe_size());
        assert!(bv.num_unique_zeros() + bv.num_unique_ones() == bv.universe_size());
        assert_eq!(bv.select0(bv.num_zeros()), None);
        assert_eq!(bv.select1(bv.num_ones()), None);
    }

    {
        // Check `get` behavior for all valid indices using a map from index -> count
        let mut counts = BTreeMap::new();
        for n in 0..bv.num_ones() {
            let i = bv.select1(n).unwrap();
            let count = counts.entry(i).or_insert(0);
            *count += 1;
        }
        for i in 0..bv.universe_size() {
            assert_eq!(bv.get(i), counts.get(&i).copied().unwrap_or(0));
        }

        let bv = bv.clone();
        assert!(catch_unwind(move || { bv.get(bv.num_zeros() + bv.num_ones()) }).is_err());
    }
}
