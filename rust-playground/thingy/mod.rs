// This was transplanted from made-of-bits; it implements a test idea for
// spatial queries in 2D for (x, y, id) data with a separate array storing element IDs.
// The idea is to store the scatterplot data in bit-reversed ID order, and store the ids
// based on the XY coordinates in Morton order. Then you can do a spatial query over the 
// IDs using the old-school Tropf method for decomposing a query rectangle into xy-ranges,
// or do spatial count queries on the xy data directly for visualizing bin counts.

/*

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

// todo: (x0, y0), (x1, y1)?
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

*/

use crate::DenseBitVecOptions;
use crate::{
    bitvec::{
        dense::DenseBitVec,
        sparse::{SparseBitVec, SparseBitVecBuilder},
        MultiBitVec, MultiBitVecBuilder,
    },
    waveletmatrix::WaveletMatrix,
    zorder,
};
use std::ops::Range;
use std::{
    collections::{BTreeMap, HashSet},
    ops::RangeInclusive,
};

pub struct Thingy {
    pub codes: WaveletMatrix<DenseBitVec>,
    pub ids: WaveletMatrix<DenseBitVec>,
    len: u32,
}

impl Thingy {
    pub fn new(xs: &[u32], ys: &[u32], ids: &[u32]) -> Self {
        let len = xs
            .len()
            .try_into()
            .expect("collections of size 2^32 or greater are not currently supported");
        // todo: assert lens of xs, ys, and ids are the same

        // morton codes, in input order
        let mut codes: Vec<_> = xs
            .iter()
            .copied()
            .zip(ys.iter().copied())
            .map(|(x, y)| zorder::encode2(x, y))
            .collect();

        // (index, code) in ascending bit-reversed id order, so we can use id offsets
        // from the bottommost layer of the ids wm to directly search the codes
        let mut index_codes: Vec<_> = codes.iter().copied().enumerate().collect();
        index_codes.sort_by_key(|&(i, code)| ids[i].reverse_bits());

        let sorted_codes: Vec<_> = index_codes.into_iter().map(|(i, code)| code).collect();
        let max_code = sorted_codes.iter().copied().max().unwrap_or(0);

        // (index, id) in ascending morton code order (not bit-reversed, since we do "preceding_count"
        // queries which return offsets for the original non-reversed order)
        let mut index_ids: Vec<_> = ids.iter().copied().enumerate().collect();
        index_ids.sort_by_key(|&(i, id)| codes[i]);

        let sorted_ids: Vec<_> = index_ids.into_iter().map(|(i, id)| id).collect();
        let max_id = sorted_ids.iter().copied().max().unwrap_or(0);
        // dbg!(&sorted_ids);

        let options = DenseBitVecOptions {
            rank1_samples_pow2: Some(10),
            select_samples_pow2: Some(10),
        };

        Thingy {
            codes: WaveletMatrix::new(sorted_codes, max_code, options, None),
            ids: WaveletMatrix::new(sorted_ids, max_id, options, None),
            len,
        }
    }

    pub fn ids_for_bbox(
        &self,
        x: RangeInclusive<u32>,
        y: RangeInclusive<u32>,
    ) -> BTreeMap<u32, u32> {
        let masks = self.codes.morton_masks_for_dims(2);
        let tl = zorder::encode2(*x.start(), *y.start());
        let br = zorder::encode2(*x.end(), *y.end());

        let end = self.len;

        let xy = tl..=br;

        println!("splitting {} {}", tl, br);

        // z-order ranges represented as a flat list of codes
        let range_symbols = zorder::split_bbox_2d(tl, br).unwrap();
        println!("result {:?}", &range_symbols);

        let mut ids = BTreeMap::new();

        // for each inclusive morton range
        for r in range_symbols.chunks_exact(2) {
            let lo = r[0];
            let hi = r[1];
            let r_lo = self.codes.preceding_count(0..end, lo);
            let r_hi = {
                let (preceding_count, range) = self.codes.locate(0..end, hi, 0);
                preceding_count + range.len() as u32
            };

            // count the ids for that morton range and accumulate into the ids map
            let rng = r_lo..r_hi;
            if !rng.is_empty() {
                println!(
                    "counting in range {:?} for morton range {:?}",
                    r_lo..r_hi,
                    lo..hi
                );

                let mut counts = self.ids.counts(&[rng], 0..=self.ids.max_symbol());

                for x in counts.results() {
                    println!("incrementing {:?}", x);
                    let count = x.v.end - x.v.start;
                    *ids.entry(x.v.symbol).or_insert(0) += count;
                }
            }
        }

        ids
    }

    // todo: maybe abstract out the traversal+accumulation, accepting just an array of index ranges

    pub fn counts_for_ids(&self, ids: &[u32]) -> BTreeMap<u32, u32> {
        let mut ids: Vec<_> = ids.iter().copied().collect();
        // ids.sort_by_key(|x| x.reverse_bits());
        ids.sort();

        let mut counts = BTreeMap::new();
        let mut query = time!("locate_batch", self.ids.locate_batch(&[0..self.len], &ids));
        let mut ranges: Vec<Range<u32>> = vec![];
        // dbg!(query.results().len());

        // we can tally up the number of elements in each symbol (id) to determine which
        // ids are "fully contained" in the rectangular query region for the time series
        // case where we can predetermine exactly how many ids there WOULD be in that region
        // if they were all contained (since only 1 per x value). ie. target_value == x extent;
        // then we can filter to those time series that are fully contained in the rectangle,
        // rather than (as we do currently) selecting those where a single point in the bbox
        // is sufficient for inclusion in the result set,

        for result in query.results() {
            // if let Some(last) = ranges.last_mut() {
            //     if last.end == result.val.start {
            //         last.end = result.val.end;
            //         continue;
            //     }
            // }
            ranges.push(result.v.start..result.v.end);
        }

        // dbg!(ranges.len());

        let mut traversal = time!(
            "counts_faster_maybe",
            self.codes.counts_faster_maybe(&ranges) //, 0..=self.codes.max_symbol())
        );

        // let mut traversal = time!(
        //     "counts",
        //     self.codes
        //         .counts(&ranges, 0..=self.codes.max_symbol(), None)
        // );

        for x in traversal.results() {
            let count = x.v.end - x.v.start;
            *counts.entry(x.v.symbol).or_insert(0) += count;
        }

        // dbg!(counts.len());

        counts
    }

    pub fn counts(&self) -> BTreeMap<u32, u32> {
        let mut counts = BTreeMap::new();
        // search over the entire symbol range (so we don't need to do a morton query)
        let mut traversal = self
            .codes
            .counts(&[0..self.len], 0..=self.codes.max_symbol());
        for x in traversal.results() {
            let count = x.v.end - x.v.start;
            *counts.entry(x.v.symbol).or_insert(0) += count;
        }
        counts
    }
}

//*
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::BufReader;

    #[derive(Serialize, Deserialize, Copy, Clone)]
    struct Datum {
        x: u32,
        y: u32,
        id: u32,
    }

    use std::time::Instant;

    use serde::{Deserialize, Serialize};
    use serde_json::Result;

    #[test]
    fn test_json() {

        let file = File::open("/Users/yurivish/Downloads/data (3).json").unwrap();
        let reader = BufReader::new(file);

        let mut data: Vec<Datum> = serde_json::from_reader(reader).unwrap();

        // let data = {
        //     let mut next = data.clone();
        //     next.extend_from_slice(&data);
        //     next
        // };
        // let data = {
        //     let mut next = data.clone();
        //     next.extend_from_slice(&data);
        //     next
        // };

        // let data = {
        //     let mut next = data.clone();
        //     next.extend_from_slice(&data);
        //     next
        // };
        // let mut data = {
        //     let mut next = data.clone();
        //     next.extend_from_slice(&data);
        //     next
        // };

        // use rand::seq::SliceRandom;
        // use rand::thread_rng;
        // let mut rng = thread_rng();
        // data.shuffle(&mut rng);

        println!("length: {}", data.len());

        let mut xs = vec![];
        let mut ys = vec![];
        let mut ids = vec![];
        for Datum { x, y, id } in data {
            xs.push(x);
            ys.push(y);
            ids.push(id);
        }

        let t = time!("construct", Thingy::new(&xs, &ys, &ids));

        let max_id = ids.iter().copied().max().unwrap();
        let query_ids: Vec<_> = (0..=max_id).step_by(3).collect();

        time!("counts_for_ids", t.counts_for_ids(&query_ids));

        panic!("wat")
        // serde_json::from_str(&serialized).unwrap();
    }

    // #[test]
    // fn test() {
    //     //
    //     let xs = vec![1, 2, 3, 1, 2, 3, 1, 2, 3];
    //     let ys = vec![1, 2, 3, 1, 2, 3, 1, 2, 3];
    //     let mut ids = vec![0, 0, 0, 1, 1, 1, 2, 2, 2];
    //     // ids.reverse();
    //     let t = Thingy::new(&xs, &ys, &ids);

    //     dbg!(t
    //         .ids
    //         .counts(&[0..t.len], 0..=t.ids.max_symbol(), None)
    //         .results());

    //     panic!("{:?}", t.counts_for_ids(&[0, 2]));

    //     panic!("{:?}", t.ids_for_bbox(0..=3, 0..=4));
    // }
}
// */
