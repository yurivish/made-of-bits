// todo: do a clarity pass on these tests - they're somewhat hard to follow
use crate::{
    bits::BASIC_BLOCK_SIZE,
    bitvec::{BitVec, BitVecBuilder},
};
use std::{
    collections::BTreeMap,
    panic::{catch_unwind, UnwindSafe},
};

pub(crate) fn test_bit_vec_builder<T: BitVecBuilder>()
where
    T::Target: UnwindSafe,
{
    test_bit_vec(T::new(0).build());

    // large enough to span many blocks
    let universe_size = BASIC_BLOCK_SIZE * 10;
    // save time by only testing with every `step`-th bit set
    {
        // one bit set

        for bit_index in (0..universe_size).step_by(((BASIC_BLOCK_SIZE >> 1) - 1) as usize) {
            let mut builder = T::new(universe_size);
            builder.one(bit_index);
            let bv = builder.build();
            test_bit_vec(bv);

            // 🌶️
            // todo: port more one bit and two bit tests
        }
    }
}

// 🌶️
// todo: port basic property tests with fisher yates shuffle for regular and multi bitvecs

pub(crate) fn test_bit_vec<T: BitVec + UnwindSafe>(bv: T) {
    {
        let bv = bv.clone();
        assert!(catch_unwind(move || { bv.get(bv.num_zeros() + bv.num_ones()) }).is_err());
    }

    // Run an adjusted set of tests in the case of multiplicity.
    // In particular, all of the bit vectors that allow multiplicity
    // Only allow it for 1 bits and disallow duplicate 0-bits.
    //
    // !!! Note: We do not test rank0 / select0 in the multiplicity case, since
    // these vectors typically do not expose efficient operations on zeros.
    assert!(bv.num_unique_zeros() + bv.num_unique_ones() == bv.universe_size());
    assert!(bv.num_zeros() + bv.num_ones() >= bv.universe_size());

    // Rank before any element should be zero
    assert_eq!(bv.rank1(0), 0);
    assert_eq!(bv.rank1(bv.num_zeros() + bv.num_ones() + 1), bv.num_ones());

    if bv.has_rank0() {
        assert_eq!(bv.rank0(0), 0);
        assert_eq!(bv.rank0(bv.num_zeros() + bv.num_ones() + 1), bv.num_zeros());
    }

    for n in 0..bv.num_ones() {
        // Verify that rank1(select1(n)) === n
        let select1 = bv.select1(n).unwrap();
        assert!(bv.rank1(select1) == n);
        assert!(bv.rank1(select1 + 1) == n + 1);
        // Check `get` behavior for valid indices
        assert_eq!(bv.get(select1), 1);
    }

    if bv.has_rank0() && bv.has_select0() {
        for n in 0..bv.num_zeros() {
            // Verify that rank0(select0(n)) === n
            let select0 = bv.select0(n).unwrap();
            assert!(bv.rank0(select0) == n);
            assert!(bv.rank0(select0 + 1) == n + 1);
            // Check `get` behavior for valid indices
            assert_eq!(bv.get(select0), 0);
        }
    }

    // Check `get` behavior for all valid indices.
    // We run this test last because the default implementation of `get`
    // relies on `rank1`, and thus it is useful to specifically test `rank1` before
    // running the test for `get`.
    let mut counts = BTreeMap::new();
    for n in 0..bv.num_ones() {
        let i = bv.select1(n).unwrap();
        let count = counts.entry(i).or_insert(0);
        *count += 1;
    }
    for (&i, &count) in counts.iter() {
        assert_eq!(bv.get(i), count);
    }
    // Assert all universe elements missing from the map are 0-bits
    for i in 0..bv.universe_size() {
        if !counts.contains_key(&i) {
            assert_eq!(bv.get(i), 0);
        }
    }

    if !bv.has_multiplicity() {
        // Perform more stringent checks when we know that multiplicity is not in play
        assert!(bv.num_zeros() + bv.num_ones() == bv.universe_size());
        assert_eq!(bv.select0(bv.num_zeros()), None);
        assert_eq!(bv.select1(bv.num_ones()), None);
    }
}
