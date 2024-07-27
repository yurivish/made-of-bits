use crate::bitvecs::multi::MultiBitVec;
use crate::bitvecs::multi::MultiBitVecBuilder;
use crate::bitvecs::rle::RLEBitVec;
use crate::bitvecs::rle::RLEBitVecBuilder;
use crate::bitvecs::sparse::SparseBitVec;
use crate::bitvecs::sparse::SparseBitVecBuilder;
use crate::{
    bitvec::{BitVec, BitVecBuilder},
    bitvecs::sortedarray::{SortedArrayBitVec, SortedArrayBitVecBuilder},
};
use to_js::{allocate, js, to_owned};

use crate::bitvecs::dense::{DenseBitVec, DenseBitVecBuilder};

macro_rules! export_bitvec {
    ($name_prefix:expr, $builder_type:ident, $bitvec_type:ident) => {
        // BitVecBuilder
        //

        #[js(name_prefix = $name_prefix)]
        fn builder_new(universe_size: u32) -> *mut $builder_type {
            allocate($builder_type::new(universe_size))
        }

        #[js(name_prefix = $name_prefix)]
        fn builder_one(x: &mut $builder_type, bit_index: u32) {
            x.one(bit_index)
        }

        #[js(name_prefix = $name_prefix)]
        fn builder_one_count(x: &mut $builder_type, bit_index: u32, count: u32) {
            x.one_count(bit_index, count)
        }

        #[js(name_prefix = $name_prefix)]
        fn builder_build(x: *mut $builder_type) -> *mut $bitvec_type {
            allocate(to_owned(x).build())
        }

        #[js(name_prefix = $name_prefix)]
        fn builder_drop(x: *mut $builder_type) -> () {
            to_owned(x);
        }

        // BitVec
        //

        #[js(name_prefix = $name_prefix)]
        fn num_ones(x: &$bitvec_type) -> u32 {
            x.num_ones()
        }

        #[js(name_prefix = $name_prefix)]
        fn num_zeros(x: &$bitvec_type) -> u32 {
            x.num_zeros()
        }

        #[js(name_prefix = $name_prefix)]
        fn universe_size(x: &$bitvec_type) -> u32 {
            x.universe_size()
        }

        #[js(name_prefix = $name_prefix)]
        fn has_multiplicity(x: &$bitvec_type) -> bool {
            x.has_multiplicity()
        }

        #[js(name_prefix = $name_prefix)]
        fn num_unique_zeros(x: &$bitvec_type) -> u32 {
            x.num_unique_zeros()
        }

        #[js(name_prefix = $name_prefix)]
        fn num_unique_ones(x: &$bitvec_type) -> u32 {
            x.num_unique_ones()
        }

        #[js(name_prefix = $name_prefix)]
        fn get(x: &$bitvec_type, bit_index: u32) -> u32 {
            x.get(bit_index)
        }

        #[js(name_prefix = $name_prefix)]
        fn rank1(x: &$bitvec_type, bit_index: u32) -> u32 {
            x.rank1(bit_index)
        }

        #[js(name_prefix = $name_prefix)]
        fn rank0(x: &$bitvec_type, bit_index: u32) -> u32 {
            x.rank0(bit_index)
        }

        #[js(name_prefix = $name_prefix)]
        fn select1(x: &$bitvec_type, n: u32) -> Option<u32> {
            x.select1(n)
        }

        #[js(name_prefix = $name_prefix)]
        fn select0(x: &$bitvec_type, n: u32) -> Option<u32> {
            x.select0(n)
        }

        #[js(name_prefix = $name_prefix)]
        fn has_rank0(x: &$bitvec_type) -> bool {
            x.has_rank0()
        }

        #[js(name_prefix = $name_prefix)]
        fn has_select0(x: &$bitvec_type) -> bool {
            x.has_select0()
        }

        #[js(name_prefix = $name_prefix)]
        fn drop(x: *mut $bitvec_type) -> () {
            to_owned(x);
        }
    };
}

export_bitvec!("sorted_array_", SortedArrayBitVecBuilder, SortedArrayBitVec);
export_bitvec!("dense_", DenseBitVecBuilder, DenseBitVec);
export_bitvec!("sparse_", SparseBitVecBuilder, SparseBitVec);
export_bitvec!("rle_", RLEBitVecBuilder, RLEBitVec);
export_bitvec!("multi_", MultiBitVecBuilder, MultiBitVec);
