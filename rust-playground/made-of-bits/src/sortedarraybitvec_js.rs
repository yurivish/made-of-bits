use crate::{
    bitvec::{BitVec, BitVecBuilder},
    sortedarraybitvec::{SortedArrayBitVec, SortedArrayBitVecBuilder},
};
use to_js::{allocate, js, to_owned};

// Builder
//

#[js]
fn sorted_array_bit_vec_builder_new(universe_size: u32) -> *mut SortedArrayBitVecBuilder {
    allocate(SortedArrayBitVecBuilder::new(universe_size))
}

#[js]
fn sorted_array_bit_vec_builder_drop(x: *mut SortedArrayBitVecBuilder) -> () {
    to_owned(x);
}

#[js]
fn sorted_array_bit_vec_builder_one(x: &mut SortedArrayBitVecBuilder, bit_index: u32) {
    x.one(bit_index)
}

#[js]
fn sorted_array_bit_vec_builder_one_count(
    x: &mut SortedArrayBitVecBuilder,
    bit_index: u32,
    count: u32,
) {
    x.one_count(bit_index, count)
}

#[js]
fn sorted_array_bit_vec_builder_build(x: *mut SortedArrayBitVecBuilder) -> *mut SortedArrayBitVec {
    allocate(to_owned(x).build())
}

// BitVec
//

#[js]
fn sorted_array_bit_vec_drop(x: *mut SortedArrayBitVec) -> () {
    to_owned(x);
}

#[js]
fn sorted_array_bit_vec_num_ones(x: &SortedArrayBitVec) -> u32 {
    x.num_ones()
}

#[js]
fn sorted_array_bit_vec_num_zeros(x: &SortedArrayBitVec) -> u32 {
    x.num_zeros()
}

#[js]
fn sorted_array_bit_vec_universe_size(x: &SortedArrayBitVec) -> u32 {
    x.universe_size()
}

#[js]
fn sorted_array_bit_vec_has_multiplicity(x: &SortedArrayBitVec) -> bool {
    x.has_multiplicity()
}

#[js]
fn sorted_array_bit_vec_num_unique_zeros(x: &SortedArrayBitVec) -> u32 {
    x.num_unique_zeros()
}

#[js]
fn sorted_array_bit_vec_num_unique_ones(x: &SortedArrayBitVec) -> u32 {
    x.num_unique_ones()
}

#[js]
fn sorted_array_bit_vec_get(x: &SortedArrayBitVec, bit_index: u32) -> u32 {
    x.get(bit_index)
}

#[js]
fn sorted_array_bit_vec_rank1(x: &SortedArrayBitVec, bit_index: u32) -> u32 {
    x.rank1(bit_index)
}

#[js]
fn sorted_array_bit_vec_rank0(x: &SortedArrayBitVec, bit_index: u32) -> u32 {
    x.rank0(bit_index)
}

#[js]
fn sorted_array_bit_vec_select1(x: &SortedArrayBitVec, n: u32) -> Option<u32> {
    x.select1(n)
}

#[js]
fn sorted_array_bit_vec_select0(x: &SortedArrayBitVec, n: u32) -> Option<u32> {
    x.select0(n)
}

#[js]
fn sorted_array_bit_vec_has_rank0(x: &SortedArrayBitVec) -> bool {
    x.has_rank0()
}

#[js]
fn sorted_array_bit_vec_has_select0(x: &SortedArrayBitVec) -> bool {
    x.has_select0()
}
