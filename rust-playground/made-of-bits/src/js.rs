use crate::thingy::Thingy;
use crate::zorder;
use crate::zorder::decode2x;
use std::collections::BTreeMap;
use to_js::Stash;
use to_js::U32Pair;

use crate::bitvec::multi::Multi;
use crate::bitvec::multi::MultiBuilder;
use crate::bitvec::rle::RLEBitVec;
use crate::bitvec::rle::RLEBitVecBuilder;
use crate::bitvec::sparse::SparseBitVec;
use crate::bitvec::sparse::SparseBitVecBuilder;
use crate::bitvec::BitVecBuilderOf;
use crate::bitvec::BitVecOf;
use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;
use crate::waveletmatrix::WaveletMatrix;
use crate::{
    bitvec::array::{ArrayBitVec, ArrayBitVecBuilder},
    bitvec::{BitVec, BitVecBuilder},
};
use to_js::{allocate, js, to_owned, Dynamic};

use crate::bitvec::dense::{DenseBitVec, DenseBitVecBuilder};

/// This macro takes as arguments a function name prefix (eg. "dense_"),
/// a BitVecBuilder type name (eg. DenseBitVecBuilder), and a BitVec
/// type name (eg. DenseBitVec), and builds a set of to_js exports for
/// the builder and bit vector methods.
macro_rules! export_bitvec {
    ($name_prefix:expr, $builder_type:ty, $bitvec_type:ty) => {
        // BitVecBuilder
        //

        #[js(name_prefix = $name_prefix)]
        fn builder_new(universe_size: u32) -> *mut $builder_type {
            allocate(<$builder_type>::new(universe_size))
        }

        #[js(name_prefix = $name_prefix)]
        fn builder_one(x: &mut $builder_type, bit_index: u32) {
            x.one(bit_index)
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
        fn get(x: &$bitvec_type, bit_index: u32) -> u32 {
            x.get(bit_index)
        }

        #[js(name_prefix = $name_prefix)]
        fn universe_size(x: &$bitvec_type) -> u32 {
            x.universe_size()
        }

        #[js(name_prefix = $name_prefix)]
        fn num_ones(x: &$bitvec_type) -> u32 {
            x.num_ones()
        }

        #[js(name_prefix = $name_prefix)]
        fn num_zeros(x: &$bitvec_type) -> u32 {
            x.num_zeros()
        }

        #[js(name_prefix = $name_prefix)]
        fn drop(x: *mut $bitvec_type) -> () {
            to_owned(x);
        }
    };
}

/// Same as above, but exporting methods for a MultiBitVec.
macro_rules! export_multibitvec {
    ($name_prefix:expr, $builder_type:ty, $bitvec_type:ty) => {
        // BitVecBuilder
        //

        #[js(name_prefix = $name_prefix)]
        fn builder_new(universe_size: u32) -> *mut $builder_type {
            allocate(<$builder_type>::new(universe_size))
        }

        #[js(name_prefix = $name_prefix)]
        fn builder_ones(x: &mut $builder_type, bit_index: u32, count: u32) {
            x.ones(bit_index, count);
        }

        #[js(name_prefix = $name_prefix)]
        fn builder_build(x: *mut $builder_type) -> *mut $bitvec_type {
            allocate(to_owned(x).build())
        }

        #[js(name_prefix = $name_prefix)]
        fn builder_drop(x: *mut $builder_type) -> () {
            to_owned(x);
        }

        // MultiBitVec
        //

        #[js(name_prefix = $name_prefix)]
        fn get(x: &$bitvec_type, bit_index: u32) -> u32 {
            x.get(bit_index)
        }

        #[js(name_prefix = $name_prefix)]
        fn rank1(x: &$bitvec_type, bit_index: u32) -> u32 {
            x.rank1(bit_index)
        }

        #[js(name_prefix = $name_prefix)]
        fn select1(x: &$bitvec_type, n: u32) -> Option<u32> {
            x.select1(n)
        }

        #[js(name_prefix = $name_prefix)]
        fn universe_size(x: &$bitvec_type) -> u32 {
            x.universe_size()
        }

        #[js(name_prefix = $name_prefix)]
        fn num_ones(x: &$bitvec_type) -> u32 {
            x.num_ones()
        }

        #[js(name_prefix = $name_prefix)]
        fn num_zeros(x: &$bitvec_type) -> u32 {
            x.num_zeros()
        }

        #[js(name_prefix = $name_prefix)]
        fn num_unique_ones(x: &$bitvec_type) -> u32 {
            x.num_unique_ones()
        }

        #[js(name_prefix = $name_prefix)]
        fn drop(x: *mut $bitvec_type) -> () {
            to_owned(x);
        }
    };
}

// export_bitvec!("dense_", DenseBitVecBuilder, DenseBitVec);
// export_bitvec!("rle_", RLEBitVecBuilder, RLEBitVec);

// export_multibitvec!(
//     "multi_",
//     MultiBuilder<DenseBitVecBuilder>,
//     Multi<DenseBitVec>
// );
// export_multibitvec!("sparse_", ArrayBitVecBuilder, ArrayBitVec);
export_multibitvec!("array_", SparseBitVecBuilder, SparseBitVec);

#[js]
fn u32_slice(len: usize) -> *mut Box<[u32]> {
    allocate(vec![0; len].into_boxed_slice())
}

#[js]
fn as_array(x: &Box<[u32]>) -> &[u32] {
    &x
}

#[js]
fn thingy_new(xs: *mut Box<[u32]>, ys: *mut Box<[u32]>, ids: *mut Box<[u32]>) -> *mut Thingy {
    // consume the data arguments
    let xs = *to_owned(xs);
    let ys = *to_owned(ys);
    let ids = *to_owned(ids);
    allocate(Thingy::new(&xs, &ys, &ids))
}

#[js]
fn thingy_counts(t: &Thingy) -> Dynamic {
    let mut xs = vec![];
    let mut ys = vec![];
    let mut counts = vec![];
    for (&k, &v) in t.counts().iter() {
        xs.push(zorder::decode2x(k));
        ys.push(zorder::decode2y(k));
        counts.push(v);
    }
    [Dynamic::new(xs), Dynamic::new(ys), Dynamic::new(counts)].into()
}

#[js]
fn thingy_counts_for_ids(t: &Thingy, ids: &Box<[u32]>) -> Dynamic {
    let mut xs = vec![];
    let mut ys = vec![];
    let mut counts = vec![];
    for (&k, &v) in t.counts_for_ids(ids).iter() {
        xs.push(zorder::decode2x(k));
        ys.push(zorder::decode2y(k));
        counts.push(v);
    }
    [Dynamic::new(xs), Dynamic::new(ys), Dynamic::new(counts)].into()
}

#[js]
fn thingy_ids_for_bbox(t: &Thingy, x_lo: u32, x_hi: u32, y_lo: u32, y_hi: u32) -> Stash<Vec<u32>> {
    let mut ids = vec![];
    let mut _counts = vec![];
    for (&k, &v) in t.ids_for_bbox(x_lo..=x_hi, y_lo..=y_hi).iter() {
        ids.push(k);
        _counts.push(v);
    }
    Stash::new(ids)
}

#[js]
fn thingy_num_levels(t: &Thingy) -> U32Pair {
    U32Pair([t.codes.num_levels() as u32, t.ids.num_levels() as u32])
}

// type WM = WaveletMatrix<DenseBitVec>;

// #[js]
// fn wavelet_matrix_new(data: *mut Box<[u32]>) -> *mut WM {
//     let data = *to_owned(data); // consume the data argument
//     let max_symbol = data.iter().max().copied().unwrap_or(0);
//     allocate(WaveletMatrix::<DenseBitVec>::new(data.into(), max_symbol))
// }

// #[js]
// fn wavelet_matrix_preceding_count(wm: &WM, range_lo: u32, range_hi: u32, symbol: u32) -> u32 {
//     wm.preceding_count(range_lo..range_hi, symbol)
// }

// #[js]
// fn wavelet_matrix_count(wm: &WM, range_lo: u32, range_hi: u32, symbol: u32) -> u32 {
//     wm.count(range_lo..range_hi, symbol)
// }

// #[js]
// fn wavelet_matrix_quantile(wm: &WM, range_lo: u32, range_hi: u32, k: u32) -> to_js::U32Pair {
//     // Returns (symbol, count)
//     to_js::U32Pair(wm.quantile(range_lo..range_hi, k).into())
// }

// #[js]
// fn wavelet_matrix_select(
//     wm: &WM,
//     range_lo: u32,
//     range_hi: u32,
//     symbol: u32,
//     k: u32,
//     ignore_bits: usize,
// ) -> Option<u32> {
//     wm.select(range_lo..range_hi, symbol, k, ignore_bits).into()
// }

// #[js]
// fn wavelet_matrix_select_last(
//     wm: &WM,
//     range_lo: u32,
//     range_hi: u32,
//     symbol: u32,
//     k: u32,
//     ignore_bits: usize,
// ) -> Option<u32> {
//     wm.select_last(range_lo..range_hi, symbol, k, ignore_bits)
//         .into()
// }

// #[js]
// fn wavelet_matrix_get(wm: &WM, index: u32) -> u32 {
//     wm.get(index)
// }

// #[js]
// fn wavelet_matrix_max_symbol(wm: &WM) -> u32 {
//     wm.max_symbol()
// }

// #[js]
// fn wavelet_matrix_simple_majority(wm: &WM, range_lo: u32, range_hi: u32) -> Option<u32> {
//     wm.simple_majority(range_lo..range_hi)
// }

// #[js]
// fn wavelet_matrix_morton_masks_for_dims(wm: &WM, dims: u32) -> *mut Box<[u32]> {
//     allocate(wm.morton_masks_for_dims(dims).into_boxed_slice())
// }

// #[js]
// fn wavelet_matrix_counts(
//     wm: &WM,
//     range_lo: u32,
//     range_hi: u32,
//     symbol_extent_lo: u32,
//     symbol_extent_hi: u32, // inclusive
//     masks: &Box<[u32]>,
// ) -> Dynamic {
//     let mut counts = wm.counts(
//         &[range_lo..range_hi],
//         symbol_extent_lo..=symbol_extent_hi,
//         Some(masks),
//     );
//     let results = counts.results();
//     // each Counts is a struct with fields `symbol`, `start`, and `end`
//     let mut symbols = vec![];
//     let mut starts = vec![];
//     let mut ends = vec![];
//     for x in results {
//         symbols.push(x.val.symbol);
//         starts.push(x.val.start);
//         ends.push(x.val.end);
//     }
//     let mut map = BTreeMap::new();
//     map.insert("symbols", Dynamic::new(symbols));
//     map.insert("starts", Dynamic::new(starts));
//     map.insert("ends", Dynamic::new(ends));
//     map.into()
// }

// // #[js]
// // pub fn counts(
// //     &self,
// //     ranges: &[Range<u32>],
// //     // note: this is inclusive because the requirement comes up in practice, eg.
// //     // a 32-bit integer can represent 3 10-bit integers, and you may want to query
// //     // for the 10-bit subcomponents separately, eg. 0..1025. But to represent 1025
// //     // in each dimension would require 33 bits. instead, inclusive extents allow
// //     // representing 0..1025 (11 bits) as 0..=1024 (10 bits).
// //     symbol_extent: RangeInclusive<u32>,
// //     masks: Option<&[u32]>,
// // ) -> Traversal<CountAll> {
