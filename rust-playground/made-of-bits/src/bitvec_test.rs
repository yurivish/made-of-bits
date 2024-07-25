use crate::bitvec::{BitVec, BitVecBuilder};
use std::panic::{catch_unwind, UnwindSafe};

pub(crate) fn test_bit_vec_builder<T: BitVecBuilder>()
where
    T::Target: UnwindSafe,
{
    let builder = T::new(0);
    let bv = builder.build();
    test_bit_vec(bv);
}

pub(crate) fn test_bit_vec(bv: impl BitVec + UnwindSafe) {
    assert!(!bv.has_multiplicity());
    assert_eq!(bv.rank1(0), 0);
    assert_eq!(bv.rank1(bv.num_zeros() + bv.num_ones() + 1), bv.num_ones());

    {
        let bv = bv.clone();
        assert!(catch_unwind(move || { bv.get(bv.num_zeros() + bv.num_ones()) }).is_err());
    }
}
