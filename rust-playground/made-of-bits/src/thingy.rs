use crate::{
    bitvec::{
        dense::DenseBitVec,
        sparse::{SparseBitVec, SparseBitVecBuilder},
        MultiBitVecBuilder,
    },
    waveletmatrix::WaveletMatrix,
    zorder,
};
use std::ops::RangeInclusive;

struct Thingy {
    codes: WaveletMatrix<DenseBitVec>,
    ids: WaveletMatrix<DenseBitVec>,
}

impl Thingy {
    fn new(xs: &[u32], ys: &[u32], ids: &[u32]) -> Self {
        // morton codes, in input order
        let mut codes: Vec<_> = xs
            .iter()
            .copied()
            .zip(ys.iter().copied())
            .map(|(x, y)| zorder::encode2(x, y))
            .collect();

        // (index, id) in ascending morton code order
        let mut index_ids: Vec<_> = ids.iter().copied().enumerate().collect();
        index_ids.sort_by_key(|&(i, id)| codes[i]);

        // (index, code) in ascending id order
        let mut index_codes: Vec<_> = codes.iter().copied().enumerate().collect();
        index_codes.sort_by_key(|&(i, code)| ids[i]);

        let sorted_codes: Vec<_> = index_codes.into_iter().map(|(i, code)| code).collect();
        let max_code = sorted_codes.last().copied().unwrap_or(0);

        let sorted_ids: Vec<_> = index_ids.into_iter().map(|(i, id)| id).collect();
        let max_id = sorted_ids.last().copied().unwrap_or(0);

        Thingy {
            codes: WaveletMatrix::<DenseBitVec>::new(sorted_codes, max_code),
            ids: WaveletMatrix::<DenseBitVec>::new(sorted_ids, max_id),
        }
    }

    fn ids_for_bbox(&self, x: RangeInclusive<u32>, y: RangeInclusive<u32>) {
        //
    }

    fn counts_for_ids(&self, ids: Option<&[u32]>) {
        if let Some(ids) = ids {
            //
        } else {
            // search over the entire symbol range
        }
    }
}
