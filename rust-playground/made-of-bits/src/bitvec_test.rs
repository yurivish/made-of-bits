use crate::{
    bits::BASIC_BLOCK_SIZE,
    bitvec::{BitVec, BitVecBuilder},
    bitvecs::sortedarray::{SortedArrayBitVec, SortedArrayBitVecBuilder},
    catch_unwind,
};
use std::{collections::BTreeMap, panic::AssertUnwindSafe};
use testresult::TestResult;

#[cfg(test)]
pub(crate) fn test_bitvec_builder<T: BitVecBuilder>() {
    // test the empty bitvec
    test_bitvec(T::new(0).build());

    // test simple case
    {
        let mut b = T::new(100);
        b.one(90);
        b.one(95);
        let bv = b.build();
        assert_eq!(bv.rank1(80), 0);
        assert_eq!(bv.rank1(93), 1);
        assert_eq!(bv.rank1(100), 2);
    }

    // large enough to span many blocks
    let universe_size = BASIC_BLOCK_SIZE * 10;
    {
        // save time by only testing with every `step`-th bit set
        let step = (BASIC_BLOCK_SIZE >> 1) - 1;

        // test with one bit set
        for bit_index in (0..universe_size).step_by(step as usize) {
            let mut b = T::new(universe_size);
            b.one(bit_index);
            let bv = b.build();
            test_bitvec(bv.clone());

            {
                // test against the same data in a sorted array bitvec
                let mut baseline_builder = SortedArrayBitVecBuilder::new(universe_size);
                baseline_builder.one(bit_index);
                let baseline = baseline_builder.build();
                test_equal(baseline, bv.clone());
            }

            assert_eq!(bv.rank1(bit_index), 0);
            assert_eq!(bv.rank1(bit_index + 1), 1);
            assert_eq!(bv.rank1(1_000_000), 1);

            assert_eq!(bv.rank0(bit_index), bit_index);
            assert_eq!(bv.rank0(bit_index + 1), bit_index);
            assert_eq!(bv.rank0(1_000_000), bv.universe_size() - 1);

            // select0
            if bv.has_select0() {
                if bit_index == 0 {
                    assert_eq!(bv.select0(0), Some(1));
                } else {
                    assert_eq!(bv.select0(0), Some(0));
                    assert_eq!(bv.select0(bit_index - 1), Some(bit_index - 1));
                }
            }

            if bit_index == bv.universe_size() - 1 {
                // if we're at the final index, there is no corresponding 0- or 1-bit
                if bv.has_select0() {
                    assert_eq!(bv.select0(bit_index), None);
                }
                assert_eq!(bv.select1(bit_index), None);
            } else {
                if bv.has_select0() {
                    assert_eq!(bv.select0(bit_index), Some(bit_index + 1));
                }
            }

            // select1
            assert_eq!(bv.select1(0), Some(bit_index));
            assert_eq!(bv.select1(1), None);
        }

        for bit_index_1 in (0..universe_size).step_by(step as usize) {
            for bit_index_2 in (bit_index_1 + step..universe_size).step_by(step as usize) {
                let mut b = T::new(universe_size);
                b.one(bit_index_1);
                b.one(bit_index_2);
                let bv = b.build();
                test_bitvec(bv.clone());

                {
                    // test against the same data in a sorted array bitvec
                    let mut baseline_builder = SortedArrayBitVecBuilder::new(universe_size);
                    baseline_builder.one(bit_index_1);
                    baseline_builder.one(bit_index_2);
                    let baseline = baseline_builder.build();
                    test_equal(baseline, bv.clone());
                }

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
                if bv.has_select0() {
                    // with 2 bits the edge cases are complex to express, so just test the first element
                    assert_eq!(
                        bv.select0(0),
                        Some(
                            (bit_index_1 == 0) as u32
                                + (bit_index_1 == 0 && bit_index_2 == 1) as u32
                        )
                    );
                }

                // select1
                assert_eq!(bv.select1(0), Some(bit_index_1));
                assert_eq!(bv.select1(1), Some(bit_index_2));
                assert_eq!(bv.select1(2), None);
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn test_equal(a: SortedArrayBitVec, b: impl BitVec) {
    // hack around the weird support for multiplicity for now
    assert_eq!(a.num_zeros(), b.num_zeros());
    assert_eq!(a.num_ones(), b.num_ones());
    assert_eq!(a.num_unique_zeros(), b.num_unique_zeros());
    assert_eq!(a.num_unique_ones(), b.num_unique_ones());
    assert_eq!(a.universe_size(), b.universe_size());
    // assert_eq!(a.has_multiplicity(), b.has_multiplicity());

    for i in 0..a.universe_size() {
        assert_eq!(a.rank1(i), b.rank1(i));
    }

    if a.has_rank0() && b.has_rank0() {
        for i in 0..a.universe_size() {
            assert_eq!(a.rank0(i), b.rank0(i));
        }
    };

    for n in 0..a.num_ones() {
        assert_eq!(a.select1(n), b.select1(n));
    }

    if a.has_select0() && b.has_select0() {
        for n in 0..a.num_zeros() {
            assert_eq!(a.select0(n), b.select0(n));
        }
    };
}

#[cfg(test)]
pub(crate) fn test_bitvec<T: BitVec>(bv: T) {
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
        if !bv.has_multiplicity() {
            assert!(bv.rank1(select1) == n);
            assert!(bv.rank1(select1 + 1) == n + 1);
        } else {
            assert!(bv.rank1(select1) <= n);
            assert!(bv.rank1(select1 + 1) <= n + bv.get(select1));
        }

        // the rank at a next position is the rank at the current position,
        // plus the number of ones at the current position (given by `get`).
        assert!(bv.rank1(select1 + 1) == bv.rank1(select1) + bv.get(select1));
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
        // Perform some exact checks when we know that multiplicity is not in play
        assert!(bv.num_zeros() + bv.num_ones() == bv.universe_size());
        assert!(bv.num_unique_zeros() + bv.num_unique_ones() == bv.universe_size());
        if bv.has_select0() {
            assert_eq!(bv.select0(bv.num_zeros()), None);
        }
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
        catch_unwind(move || bv.get(bv.num_zeros() + bv.num_ones())).unwrap_err();
    }
}

/// Generate bitvectors with arbitrary densities of 1-bits and run them through our basic test_bitvec test function.
pub(crate) fn property_test_bitvec_builder<T: BitVecBuilder>(
    seed: Option<u64>,
    budget_ms: Option<u64>,
    minimize: bool,
) {
    use arbtest::{arbitrary, arbtest};

    fn property<T: BitVecBuilder>(u: &mut arbitrary::Unstructured) -> arbitrary::Result<()> {
        let ones_percent = u.int_in_range(0..=100)?; // density
        let universe_size = u.arbitrary_len::<u32>()? as u32;
        let mut b = T::new(universe_size);
        // test against the same data in a sorted array bitvec
        let mut baseline_builder = SortedArrayBitVecBuilder::new(universe_size);
        // construct with multiplicity some of the time
        let with_multiplicity = if T::Target::supports_multiplicity() {
            u.ratio(1, 3)?
        } else {
            false
        };
        for i in 0..universe_size {
            if u.int_in_range(0..=100)? < ones_percent {
                let count = if with_multiplicity {
                    u.int_in_range(0..=10)?
                } else {
                    1
                };
                b.one_count(i, count);
                baseline_builder.one_count(i, count);
            }
        }
        let bv = b.build();
        let baseline = baseline_builder.build();
        test_equal(baseline, bv.clone());
        test_bitvec(bv);
        return Ok(());
    }

    let mut test = arbtest(property::<T>);

    if let Some(seed) = seed {
        test = test.seed(seed)
    }

    if let Some(budget_ms) = budget_ms {
        test = test.budget_ms(budget_ms)
    }

    if minimize {
        test = test.minimize()
    }
}
