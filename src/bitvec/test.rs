use crate::{
    bitvec::{array::ArrayBitVecBuilder, rle::RLEBitVecBuilder},
    bitvec::{BitVec, BitVecBuilder, BitVecBuilderOf, MultiBitVec, MultiBitVecBuilder},
    panics,
};
use arbtest::{arbitrary, arbtest, ArbTest};
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
// - Default options are used everywhere; we should be able to generate and test arbitrary options.

/// Top-level functions for testing the BitVec and MultiBitVec interfaces

pub(crate) fn test_bitvec_builder<T: BitVecBuilder>() {
    // Spot tests (manually-written individual test cases for basic checking)
    spot_test_bitvec_builder::<T>();
    // Boundary-size + density sweep at the universe-size powers-of-2 edges.
    // (cheap, deterministic, runs before the random sweeps so the bisect target is clean
    // when something breaks during a port.)
    boundary_sweep_test_bitvec_builder::<T>();
    // Sweep tests (exhaustive sweeps of tractable parameter spaces, checking against ArrayBitVec)
    sweep_test_bitvec_builder::<T>();
    // Property tests (randomized tests of larger parameter spaces, checking against ArrayBitVec)
    prop_test_bitvec_builder::<T>();
}

/// Doesn't currently run sweep tests, relying on randomized testing plus the passing BitVec sweep tests
/// to find bugs.
pub(crate) fn test_multibitvec_builder<T: MultiBitVecBuilder>() {
    // run bitvec tests
    test_bitvec_builder::<BitVecBuilderOf<T>>();

    // run multibitvec tests (no sweep tests yet!)
    spot_test_multibitvec_builder::<T>();
    prop_test_multibitvec_builder::<T>();
}

// BitVec
//

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

fn naive_rank1_batch(v: impl BitVec, bit_indices: &[u32]) -> Vec<u32> {
    bit_indices.iter().copied().map(|i| v.rank1(i)).collect()
}

fn naive_multi_rank1_batch(v: impl MultiBitVec, bit_indices: &[u32]) -> Vec<u32> {
    bit_indices.iter().copied().map(|i| v.rank1(i)).collect()
}

/// Spot tests for BitVec
pub(crate) fn spot_test_bitvec_builder<T: BitVecBuilder>() {
    {
        // empty bitvec
        let bv = T::new(0, Default::default()).build();

        assert_eq!(bv.rank1(0), 0);
        assert_eq!(bv.rank1(1), 0);
        assert_eq!(bv.rank1(u32::MAX), 0);

        assert_eq!(bv.rank0(0), 0);
        assert_eq!(bv.rank0(1), 0);
        assert_eq!(bv.rank0(u32::MAX), 0);

        assert_eq!(bv.select1(0), None);
        assert_eq!(bv.select1(1), None);
        assert_eq!(bv.select1(u32::MAX), None);

        assert_eq!(bv.select0(0), None);
        assert_eq!(bv.select0(1), None);
        assert_eq!(bv.select0(u32::MAX), None);

        assert_eq!(bv.num_ones(), 0);
        assert_eq!(bv.num_zeros(), 0);
        assert_eq!(bv.universe_size(), 0);

        // all-zero bitvec
        let bv = T::new(100, Default::default()).build();

        assert_eq!(bv.rank1(0), 0);
        assert_eq!(bv.rank1(1), 0);
        assert_eq!(bv.rank1(u32::MAX), 0);

        assert_eq!(bv.rank0(0), 0);
        assert_eq!(bv.rank0(1), 1);
        assert_eq!(bv.rank0(u32::MAX), 100);

        assert_eq!(bv.select1(0), None);
        assert_eq!(bv.select1(1), None);
        assert_eq!(bv.select1(u32::MAX), None);

        assert_eq!(bv.select0(0), Some(0));
        assert_eq!(bv.select0(1), Some(1));
        assert_eq!(bv.select0(u32::MAX), None);

        assert_eq!(bv.num_ones(), 0);
        assert_eq!(bv.num_zeros(), 100);
        assert_eq!(bv.universe_size(), 100);
    }

    {
        // builder allows but ignores multiplicity (idempotency)
        let mut b = T::new(10, Default::default());
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
        let mut b = T::new(100, Default::default());
        assert!(panics(|| b.one(100)));
    }

    {
        // bitvec gives correct answers for some basic rank and select queries.
        // we use a universe size of 70 since it's enough to cover a few basic blocks.

        let mut b = T::new(70, Default::default());
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
                    0 | 31 | 32 | 68 => 1,
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
        // check rank1_batch
        let mut b = T::new(1000, Default::default());
        for i in [1, 10, 11, 50, 72, 205] {
            b.one(i);
        }
        let v = b.build();
        let mut bit_indices = [1, 4, 6, 10, 40, 50, 51, 100, 500, 5000];
        let _bit_indices = bit_indices.clone();
        v.rank1_batch(&mut bit_indices);
        let out = naive_rank1_batch(v, &_bit_indices);
        assert_eq!(bit_indices, *out);
    }

    {
        // check that maximum-size bitvecs are constructible and queryable
        let bv = if type_name::<T>() == type_name::<RLEBitVecBuilder>() {
            // The RLEBitVec rejects a universe size of u32::MAX
            // due to the fact that it needs to place a 1 into one of its inner
            // bit vectors at index `universe_size`.
            assert!(panics(|| T::new(u32::MAX, Default::default()).build()));

            let mut b = T::new(u32::MAX - 1, Default::default());
            // can add a bit at the maximum allowed index
            b.one(u32::MAX - 2);
            // cannot add one beyond that
            assert!(panics(|| b.one(u32::MAX - 1)));
            // builds without panic
            b.build()
        } else {
            let mut b = T::new(u32::MAX, Default::default());
            // can add a bit at the maximum allowed index
            b.one(u32::MAX - 1);
            // cannot add one beyond that
            assert!(panics(|| b.one(u32::MAX)));
            // builds without panic
            b.build()
        };
        assert_eq!(bv.num_ones(), 1);
        assert_eq!(bv.rank1(u32::MAX), 1);
    }
}

/// Property tests for BitVec
pub(crate) fn prop_test_bitvec_builder<T: BitVecBuilder>(
) -> ArbTest<impl FnMut(&mut arbitrary::Unstructured<'_>) -> arbitrary::Result<()>> {
    arbtest(|u| {
        let universe_size = u.arbitrary_len::<u32>()? as u32;
        let ones = arbitrary_ones(u, universe_size)?;
        // todo: can we do arbitrary options?
        test_bitvec::<T>(universe_size, Default::default(), ones);
        Ok(())
    })
}

/// Boundary-universe-size sweep: powers-of-2 boundaries and the values immediately
/// adjacent, crossed with a small set of density patterns.
pub(crate) fn boundary_sweep_test_bitvec_builder<T: BitVecBuilder>() {
    // Sizes that exercise the block/sample-boundary arithmetic. Keep this list short —
    // each (size, pattern) pair runs the full cross-check.
    const SIZES: &[u32] = &[0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1023, 1024, 1025];

    for &size in SIZES {
        for pattern in BoundaryPattern::ALL {
            // RLEBitVecBuilder rejects universe size u32::MAX; everything else is fine here.
            // Patterns that would put ones at out-of-range positions for size 0 are skipped.
            let ones = pattern.ones(size);
            test_bitvec::<T>(size, Default::default(), ones);
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum BoundaryPattern {
    Empty,
    SingleFirst,
    SingleLast,
    AllOnes,
    Alternating,
    Every8th,
    Sparse, // every 7-th, just to drift off block boundaries
}

impl BoundaryPattern {
    const ALL: [Self; 7] = [
        Self::Empty,
        Self::SingleFirst,
        Self::SingleLast,
        Self::AllOnes,
        Self::Alternating,
        Self::Every8th,
        Self::Sparse,
    ];

    fn ones(self, size: u32) -> Vec<u32> {
        if size == 0 {
            return vec![];
        }
        match self {
            Self::Empty => vec![],
            Self::SingleFirst => vec![0],
            Self::SingleLast => vec![size - 1],
            Self::AllOnes => (0..size).collect(),
            Self::Alternating => (0..size).step_by(2).collect(),
            Self::Every8th => (0..size).step_by(8).collect(),
            Self::Sparse => (0..size).step_by(7).collect(),
        }
    }
}

/// Sweep tests for BitVec
pub(crate) fn sweep_test_bitvec_builder<T: BitVecBuilder>() {
    let mut gen = Gen::new();
    // Exhaustively generate all 1-length and 2-length ones arrays
    // and individually test bitvectors built from them.
    let universe_size = 5 * 32;
    while !gen.done() {
        // NOTE: Does double the work necessary since `gen_elts` generates both [x, y] and [y, x].
        // TODO: figure out how to generate each unique combination only once (ignoring order).
        let ones: Vec<u32> = gen
            .gen_elts(2, universe_size as usize - 1) // note the inclusive upper bound
            .map(|x| x as u32)
            .collect();
        test_bitvec::<T>(universe_size, Default::default(), ones);
    }
}

/// Equality tests for BitVec, checking against an ArrayBitVec
pub(crate) fn test_bitvec<T: BitVecBuilder>(
    universe_size: u32,
    options: T::Options,
    ones: Vec<u32>,
) {
    // a is the baseline and b is the candidate bitvector under test
    let a =
        BitVecBuilderOf::<ArrayBitVecBuilder>::from_ones(universe_size, Default::default(), &ones);
    let b = T::from_ones(universe_size, options, &ones);

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

    let mut bit_indices: Vec<_> = (0..universe_size).step_by(3).collect();
    let a_out = naive_rank1_batch(a, &bit_indices);
    let naive_out = naive_rank1_batch(b.clone(), &bit_indices);
    b.rank1_batch(&mut bit_indices);
    assert_eq!(a_out, bit_indices);
    assert_eq!(a_out, naive_out);

    // Explicit invariants. The cross-check above already implies these (since the
    // ArrayBitVec reference holds them), but asserting them directly gives a much
    // sharper diagnostic when the underlying impl regresses.
    assert_invariants(&b);
}

/// Explicit invariant assertions that any `BitVec` implementation must satisfy.
/// Cheap O(universe_size) sweeps; safe to call from every `test_bitvec` invocation.
pub(crate) fn assert_invariants<V: BitVec>(v: &V) {
    let n = v.universe_size();

    // num_ones + num_zeros == universe_size.
    assert_eq!(
        v.num_ones() + v.num_zeros(),
        n,
        "num_ones + num_zeros != universe_size for {}",
        type_name::<V>(),
    );

    // rank1 + rank0 == bit_index, for every position.
    // (Also implicitly: rank1 monotone non-decreasing, rank0 monotone non-decreasing.)
    let mut prev_rank1 = 0u32;
    let mut prev_rank0 = 0u32;
    for i in 0..n {
        let r1 = v.rank1(i);
        let r0 = v.rank0(i);
        assert_eq!(r0 + r1, i, "rank0+rank1 != i at i={i}");
        assert!(r1 >= prev_rank1, "rank1 not monotone at i={i}");
        assert!(r0 >= prev_rank0, "rank0 not monotone at i={i}");
        prev_rank1 = r1;
        prev_rank0 = r0;
    }
    // rank1(n) == num_ones, rank0(n) == num_zeros.
    assert_eq!(v.rank1(n), v.num_ones());
    assert_eq!(v.rank0(n), v.num_zeros());

    // Select monotonicity + select/rank round-trip.
    let mut prev_pos: Option<u32> = None;
    for k in 0..v.num_ones() {
        let pos = v.select1(k).unwrap_or_else(|| {
            panic!("select1({k}) returned None but num_ones={}", v.num_ones())
        });
        if let Some(p) = prev_pos {
            assert!(pos > p, "select1 not monotone at k={k}");
        }
        prev_pos = Some(pos);
        // Round-trip: rank1(select1(k)+1) > k (the k-th 1-bit lies at this position).
        assert_eq!(v.rank1(pos), k, "rank1(select1({k}))");
        assert_eq!(v.get(pos), 1, "get(select1({k}))");
    }
    // Out-of-range select returns None.
    assert_eq!(v.select1(v.num_ones()), None);

    let mut prev_pos: Option<u32> = None;
    for k in 0..v.num_zeros() {
        let pos = v.select0(k).unwrap_or_else(|| {
            panic!("select0({k}) returned None but num_zeros={}", v.num_zeros())
        });
        if let Some(p) = prev_pos {
            assert!(pos > p, "select0 not monotone at k={k}");
        }
        prev_pos = Some(pos);
        assert_eq!(v.rank0(pos), k, "rank0(select0({k}))");
        assert_eq!(v.get(pos), 0, "get(select0({k}))");
    }
    assert_eq!(v.select0(v.num_zeros()), None);
}

// MultiBitVec
//

/// Spot tests for MultiBitVec
pub(crate) fn spot_test_multibitvec_builder<T: MultiBitVecBuilder>() {
    {
        // empty bitvec
        let bv = T::new(0, Default::default()).build();
        assert_eq!(bv.num_unique_ones(), 0);
    }

    {
        // check rank1_batch
        let mut b = T::new(1000, Default::default());
        for i in [1, 10, 11, 50, 72, 205] {
            b.ones(i, 2);
        }
        let v = b.build();
        let mut bit_indices = [1, 4, 6, 10, 40, 50, 51, 100, 500, 5000];
        let _bit_indices = bit_indices.clone();
        v.rank1_batch(&mut bit_indices);
        let out = naive_multi_rank1_batch(v, &_bit_indices);
        assert_eq!(bit_indices, *out);
    }
}

/// Property tests for MultiBitVec
pub(crate) fn prop_test_multibitvec_builder<T: MultiBitVecBuilder>(
) -> ArbTest<impl FnMut(&mut arbitrary::Unstructured<'_>) -> arbitrary::Result<()>> {
    arbtest(|u| {
        let universe_size = u.arbitrary_len::<u32>()? as u32;
        let ones = arbitrary_ones(u, universe_size)?;
        dbg!(universe_size, &ones);
        // generate a random count for each 1-bit, limiting the maximum count
        // for each so that the total count doesn't risk overflowing u32.
        let counts: Vec<u32> = ones
            .iter()
            .map(|_| u.arbitrary::<u32>().map(|x| x % 100))
            .collect::<arbitrary::Result<_>>()?;
        test_multibitvec::<T>(universe_size, ones, counts);
        Ok(())
    })
}

/// Equality tests for MultiBitVec, checking against an ArrayBitVec
pub(crate) fn test_multibitvec<T: MultiBitVecBuilder>(
    universe_size: u32,
    ones: Vec<u32>,
    counts: Vec<u32>,
) {
    // a is the baseline and b is the candidate bitvector under test
    let a = ArrayBitVecBuilder::from_ones_counts(universe_size, Default::default(), &ones, &counts);
    let b = T::from_ones_counts(universe_size, Default::default(), &ones, &counts);

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

    // test with some extremely large values
    for i in universe_size * 1000..universe_size * 1001 {
        assert_eq!(a.rank1(i), b.rank1(i));
        assert_eq!(a.select1(i), b.select1(i));
        assert_eq!(a.rank1(i), b.rank1(i));
        assert_eq!(a.select1(i), b.select1(i));
    }
}
