use crate::{
    bits::BASIC_BLOCK_SIZE,
    bitvec::{BitVec, BitVecBuilder, BitVecBuilderOf, MultiBitVec, MultiBitVecBuilder},
    bitvecs::{array::ArrayBitVecBuilder, rle::RLEBitVecBuilder},
    panics,
};
use arbtest::arbitrary;
use arbtest::ArbTest;
use exhaustigen::Gen;
use std::any::type_name;

// This file contains test functions for the BitVec and MultiBitVec interfaces.
// `test_bitvec_builder` and `test_multibitvec_builder` are called from the tests of individual
// BitVec and MultiBitVec implementations.
//
// Currently un- or under-tested corners
// - test adding bits with count=0 to a multibitvec using builder.ones
// - test an all-1-bits bitvec
// - some individual bitvec types should have type-specific tests that stress specific bitvec
//   shapes and assert the appropriate size guarantees, eg. sparse (large universe), rle (runs),
//   and multi (large multiplicities)

/// Top-level function for testing the BitVec interface
/// Runs
/// - Spot tests (manually-written individual test cases for basic checking)
/// - Sweep tests (exhaustive sweeps of tractable parameter spaces, checking against ArrayBitVec)
/// - Property tests (randomized tests of larger parameter spaces, checking against ArrayBitVec)
pub(crate) fn test_bitvec_builder<T: BitVecBuilder>() {
    spot_test_bitvec_builder::<T>();
    sweep_test_bitvec_builder::<T>();
    prop_test_bitvec_builder::<T>();
}

/// Top-level function for testing the MultiBitVec interface.
/// Doesn't currently run sweep tests, relying on randomized testing plus the passing BitVec sweep tests
/// to find bugs.
pub(crate) fn test_multibitvec_builder<T: MultiBitVecBuilder>() {
    // run bitvec tests
    test_bitvec_builder::<BitVecBuilderOf<T>>();

    // run multibitvec tests
    spot_test_multibitvec_builder::<T>();
    prop_test_multibitvec_builder::<T>();
}

pub(crate) fn spot_test_multibitvec_builder<T: MultiBitVecBuilder>() {
    {
        // empty bitvec
        let bv = T::new(0).build();
        assert_eq!(bv.num_unique_zeros(), 0);
        assert_eq!(bv.num_unique_ones(), 0);
    }
}

/// Generate a vector of random 1-bit positions in 0..universe_size for property testing
fn arbitrary_ones(
    u: &mut arbitrary::Unstructured<'_>,
    universe_size: u32,
) -> arbitrary::Result<Vec<u32>> {
    if universe_size == 0 {
        Ok(vec![])
    } else {
        u.arbitrary::<Vec<u32>>()
            .map(|v| v.into_iter().map(|x| x % universe_size).collect())
    }
}

pub(crate) fn prop_test_bitvec_builder<T: BitVecBuilder>(
) -> ArbTest<impl FnMut(&mut arbitrary::Unstructured<'_>) -> arbitrary::Result<()>> {
    use arbtest::arbtest;
    arbtest(|u| {
        let universe_size = u.arbitrary_len::<u32>()? as u32 % (u32::MAX - 1);
        let ones = arbitrary_ones(u, universe_size)?;
        test_bitvec::<T>(universe_size, ones);
        Ok(())
    })
}

pub(crate) fn sweep_test_bitvec_builder<T: BitVecBuilder>() {
    let mut gen = Gen::new();
    // Exhaustively generate all 1-length and 2-length ones arrays
    // and individually test bitvectors built from them.
    let universe_size = 5 * BASIC_BLOCK_SIZE;
    while !gen.done() {
        let ones: Vec<u32> = gen
            .gen_elts(2, universe_size as usize - 1) // note the inclusive upper bound
            .map(|x| x as u32)
            .collect();
        test_bitvec::<T>(universe_size, ones);
    }
}

pub(crate) fn test_bitvec<T: BitVecBuilder>(universe_size: u32, ones: Vec<u32>) {
    // a is baseline, b is the candidate bitvector under test
    let a = BitVecBuilderOf::<ArrayBitVecBuilder>::from_ones(universe_size, &ones);
    let b = T::from_ones(universe_size, &ones);

    assert_eq!(a.num_zeros(), b.num_zeros());
    assert_eq!(a.num_ones(), b.num_ones());
    assert_eq!(a.universe_size(), b.universe_size());

    for i in 0..universe_size {
        assert_eq!(a.get(i), b.get(i));
    }

    // test with some extra values on the top of the array to ensure that out-of-bounds
    // queries are treated identically between the two options
    for i in 0..universe_size.saturating_add(10) {
        assert_eq!(a.rank1(i), b.rank1(i));
        assert_eq!(a.rank0(i), b.rank0(i));
        assert_eq!(a.select1(i), b.select1(i));
        assert_eq!(a.select0(i), b.select0(i));
    }
}

pub(crate) fn spot_test_bitvec_builder<T: BitVecBuilder>() {
    {
        // empty bitvec
        let bv = T::new(0).build();

        assert_eq!(bv.rank1(0), 0);
        assert_eq!(bv.rank1(1), 0);
        assert_eq!(bv.rank1(u32::MAX), 0);

        assert_eq!(bv.select1(0), None);
        assert_eq!(bv.select1(1), None);
        assert_eq!(bv.select1(u32::MAX), None);

        assert_eq!(bv.num_ones(), 0);
        assert_eq!(bv.num_zeros(), 0);
        assert_eq!(bv.universe_size(), 0);
    }

    {
        // builder allows but ignores multiplicity (idempotency)
        let mut b = T::new(10);
        b.one(5);
        b.one(5);
        let bv = b.build();
        assert_eq!(bv.num_ones(), 1);
        assert_eq!(bv.rank1(5), 0);
        assert_eq!(bv.rank1(6), 1);
        assert_eq!(bv.select1(0), Some(5));
    }

    {
        // builder panics on out-of-range values
        let mut b = T::new(100);
        assert!(panics(|| b.one(100)));
    }

    {
        // bitvec gives correct answers for some basic rank and select queries.
        // we use a universe size of 70 since it's enough to cover a few basic blocks.

        let mut b = T::new(70);
        b.one(0);
        b.one(31);
        b.one(32);
        b.one(68);
        let bv = b.build();

        assert_eq!(bv.num_ones(), 4);
        assert_eq!(bv.rank1(5), 1);
        assert_eq!(bv.rank1(31), 1);
        assert_eq!(bv.rank1(32), 2);
        assert_eq!(bv.rank1(33), 3);
        assert_eq!(bv.rank1(60), 3);
        assert_eq!(bv.rank1(69), 4);
        assert_eq!(bv.rank1(70), 4);

        assert_eq!(bv.select1(0), Some(0));
        assert_eq!(bv.select1(1), Some(31));
        assert_eq!(bv.select1(2), Some(32));
        assert_eq!(bv.select1(3), Some(68));

        // test that `get` is 1 at precisely the positions of the added 1-bits
        for i in 0..70 {
            assert_eq!(
                bv.get(i),
                match i {
                    0 => 1,
                    31 => 1,
                    32 => 1,
                    68 => 1,
                    _ => 0,
                }
            );
        }

        assert_eq!(bv.rank0(5), 4);
        assert_eq!(bv.rank0(31), 30);
        assert_eq!(bv.rank0(32), 30);
        assert_eq!(bv.rank0(33), 30);
        assert_eq!(bv.rank0(60), 57);
        assert_eq!(bv.rank0(69), 65);
        assert_eq!(bv.rank0(70), 66);

        assert_eq!(bv.num_zeros(), 66);
        assert_eq!(bv.select0(0), Some(1));
        assert_eq!(bv.select0(1), Some(2));
        assert_eq!(bv.select0(2), Some(3));
        assert_eq!(bv.select0(3), Some(4));
        assert_eq!(bv.select0(31), Some(34));
    }

    {
        // No BitVecs accept a universe size of u32::MAX.
        // The RLEBitVec additionally rejects a universe size of u32::MAX-2
        // due to the fact that it needs to place a 1 into one of its internal
        // bit vectors at index `universe_size`.
        // Test that construction at these limits panics, and construction
        // at just under the limit does not panic.
        assert!(panics(|| T::new(u32::MAX).build()));
        if type_name::<T>() == type_name::<RLEBitVecBuilder>() {
            assert!(panics(|| T::new(u32::MAX - 1).build()));
            T::new(u32::MAX - 2).build();
        } else {
            T::new(u32::MAX - 1).build();
        }
    }
}

pub(crate) fn prop_test_multibitvec_builder<T: MultiBitVecBuilder>(
) -> ArbTest<impl FnMut(&mut arbitrary::Unstructured<'_>) -> arbitrary::Result<()>> {
    use arbtest::arbtest;
    arbtest(|u| {
        let universe_size = u.arbitrary_len::<u32>()? as u32 % (u32::MAX - 1);
        let ones = arbitrary_ones(u, universe_size)?;
        dbg!(universe_size, &ones);
        // generate a random count for each 1-bit, limiting the maximum count
        // for each so that the total count doesn't risk overflowing u32.
        let counts: Vec<u32> = ones
            .iter()
            .map(|_| u.arbitrary::<u32>().map(|x| x % 100))
            .collect::<arbitrary::Result<Vec<_>>>()?;
        test_multibitvec::<T>(universe_size, ones, counts);
        Ok(())
    })
}

pub(crate) fn test_multibitvec<T: MultiBitVecBuilder>(
    universe_size: u32,
    ones: Vec<u32>,
    counts: Vec<u32>,
) {
    // a is baseline, b is the candidate bitvector under test
    let a = ArrayBitVecBuilder::from_ones_counts(universe_size, &ones, &counts);
    let b = T::from_ones_counts(universe_size, &ones, &counts);

    assert_eq!(a.num_zeros(), b.num_zeros());
    assert_eq!(a.num_ones(), b.num_ones());
    assert_eq!(a.universe_size(), b.universe_size());

    for i in 0..universe_size {
        assert_eq!(a.get(i), b.get(i));
    }

    // test with some extra values on the top of the array to ensure that out-of-bounds
    // queries are treated identically between the two options
    for i in 0..a.num_ones().saturating_add(10) {
        assert_eq!(a.rank1(i), b.rank1(i));
        assert_eq!(a.select1(i), b.select1(i));
        assert_eq!(a.rank1(i), b.rank1(i));
        assert_eq!(a.select1(i), b.select1(i));
    }
}
