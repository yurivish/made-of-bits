use crate::{
    bitvec::{
        dense::DenseBitVec,
        sparse::{SparseBitVec, SparseBitVecBuilder},
        MultiBitVec, MultiBitVecBuilder,
    },
    waveletmatrix::WaveletMatrix,
    zorder,
};
use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
};

struct Thingy {
    codes: WaveletMatrix<DenseBitVec>,
    ids: WaveletMatrix<DenseBitVec>,
    len: u32,
}

impl Thingy {
    fn new(xs: &[u32], ys: &[u32], ids: &[u32]) -> Self {
        let len = xs
            .len()
            .try_into()
            .expect("collections of size 2^32 or greater are not currently supported");
        // todo: assert len xs ys and ids are the same

        // morton codes, in input order
        let mut codes: Vec<_> = xs
            .iter()
            .copied()
            .zip(ys.iter().copied())
            .map(|(x, y)| zorder::encode2(x, y))
            .collect();

        // (index, code) in ascending bit-reversed id order
        let mut index_codes: Vec<_> = codes.iter().copied().enumerate().collect();
        index_codes.sort_by_key(|&(i, code)| ids[i].reverse_bits());

        let sorted_codes: Vec<_> = index_codes.into_iter().map(|(i, code)| code).collect();
        let max_code = sorted_codes.iter().copied().max().unwrap_or(0);

        // (index, id) in ascending morton code order (not bit-reversed, since we do "preceding_count" queries in the real order)
        let mut index_ids: Vec<_> = ids.iter().copied().enumerate().collect();
        index_ids.sort_by_key(|&(i, id)| codes[i]);

        let sorted_ids: Vec<_> = index_ids.into_iter().map(|(i, id)| id).collect();
        let max_id = sorted_ids.iter().copied().max().unwrap_or(0);
        dbg!(&sorted_ids);

        // todo: encode breaks in ids using a dense bit vector
        // (it can be dense since each id must appear at least once; otherwise
        //  we could use a sparse or multi<dense> bitvector)

        // codes.sort();
        // let counts: Vec<_> = std::iter::repeat(1).take(codes.len()).collect();
        // let bv =
        // SparseBitVecBuilder::from_ones_counts(*codes.last().unwrap() + 1, &*codes, &*counts);

        Thingy {
            codes: WaveletMatrix::<DenseBitVec>::new(sorted_codes, max_code),
            ids: WaveletMatrix::<DenseBitVec>::new(sorted_ids, max_id),
            // bv,
            len,
        }
    }

    fn ids_for_bbox(&self, x: RangeInclusive<u32>, y: RangeInclusive<u32>) -> HashMap<u32, u32> {
        let masks = self.codes.morton_masks_for_dims(2);
        let tl = zorder::encode2(*x.start(), *y.start());
        let br = zorder::encode2(*x.end(), *y.end());

        let end = self.len;

        let xy = tl..=br;

        println!("splitting {} {}", tl, br);

        // z-order ranges represented as a flat list of codes
        let range_symbols = zorder::split_bbox_2d(tl, br).unwrap();
        println!("result {:?}", &range_symbols);

        let mut ids = HashMap::new();

        // for each inclusive morton range
        for r in range_symbols.chunks_exact(2) {
            let lo = r[0];
            let hi = r[1];
            // dbg!(lo, hi, self.codes.max_symbol());
            // get the index range at the bottom of the codes wm,
            // which is ordered the same way as the ids
            // let r_lo = self.codes.preceding_count(0..end, lo);
            // let r_hi = {
            //     let (preceding_count, range) = self.codes.locate(0..end, hi, 0);
            //     preceding_count + range.len() as u32
            // };

            // let r_lo = self.bv.rank1(lo);
            // let r_hi = self.bv.rank1(hi) + self.bv.get(hi);
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

                let mut counts = self.ids.counts(&[rng], 0..=self.ids.max_symbol(), None);

                for x in counts.results() {
                    println!("incrementing {:?}", x);
                    let count = x.val.end - x.val.start;
                    *ids.entry(x.val.symbol).or_insert(0) += count;
                }
            }
        }

        ids
    }

    fn counts_for_ids(&self, ids: Option<&[u32]>) -> HashMap<u32, u32> {
        let mut counts = HashMap::new();

        if let Some(ids) = ids {
            // use a morton query per contiguous id range
            // we could do masked queries if we want to support zooming.
            for id in ids.iter().copied() {
                let range = self.ids.locate(0..self.len, id, 0).1;
                let mut traversal = self
                    .codes
                    .counts(&[range], 0..=self.codes.max_symbol(), None);
                for x in traversal.results() {
                    let count = x.val.end - x.val.start;
                    *counts.entry(x.val.symbol).or_insert(0) += count;
                }
            }
        } else {
            // search over the entire symbol range
            let mut traversal =
                self.codes
                    .counts(&[0..self.len], 0..=self.codes.max_symbol(), None);
            for x in traversal.results() {
                let count = x.val.end - x.val.start;
                *counts.entry(x.val.symbol).or_insert(0) += count;
            }
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        //
        let xs = vec![1, 2, 3, 1, 2, 3, 1, 2, 3];
        let ys = vec![1, 2, 3, 1, 2, 3, 1, 2, 3];
        let mut ids = vec![0, 0, 0, 1, 1, 1, 2, 2, 2];
        // ids.reverse();
        let t = Thingy::new(&xs, &ys, &ids);

        dbg!(t
            .ids
            .counts(&[0..t.len], 0..=t.ids.max_symbol(), None)
            .results());

        panic!("{:?}", t.counts_for_ids(Some(&[0, 2])));

        panic!("{:?}", t.ids_for_bbox(0..=3, 0..=4));
    }
}
