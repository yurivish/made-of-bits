//! `ByteSize` trait reporting total memory footprint (inline + heap) in bytes.
//!
//! Each implementation is hand-written (no runtime reflection). Container types
//! (`Multi<T>`, `ZeroPadded<T>`, `OnePadded<T>`, `WaveletMatrix<BV>`) are generic over
//! `T: ByteSize`. Inner buffers that aren't reachable from outside their module
//! report inline size only — a conservative under-count, tightenable by adding
//! accessors.

use crate::bitvec::BitVec;
use crate::bitvec::MultiBitVec;
use std::collections::HashMap;
use std::mem;

pub trait ByteSize {
    fn byte_size(&self) -> u64;
}

/// Approximate `HashMap<K, V>` overhead. Assumes ~16 bytes of Swiss-table bookkeeping
/// per occupied slot beyond the (K, V) payload — a lower bound at typical load factors.
fn hashmap_overhead_bytes<K, V>(map: &HashMap<K, V>) -> u64 {
    let per_entry = (mem::size_of::<K>() + mem::size_of::<V>() + 16) as u64;
    map.len() as u64 * per_entry
}

impl ByteSize for crate::bitbuf::BitBuf {
    fn byte_size(&self) -> u64 {
        mem::size_of_val(self) as u64
            + self.blocks().len() as u64 * mem::size_of::<crate::bitbuf::Block>() as u64
    }
}

impl ByteSize for crate::intbuf::IntBuf {
    fn byte_size(&self) -> u64 {
        mem::size_of_val(self) as u64
    }
}

impl ByteSize for crate::bitvec::array::ArrayBitVec {
    fn byte_size(&self) -> u64 {
        mem::size_of_val(self) as u64 + self.num_ones() as u64 * mem::size_of::<u32>() as u64
    }
}

impl ByteSize for crate::bitvec::dense::DenseBitVec {
    fn byte_size(&self) -> u64 {
        let inline = mem::size_of_val(self) as u64;
        let blocks = (self.universe_size() as u64).div_ceil(64);
        inline + blocks * mem::size_of::<crate::bitbuf::Block>() as u64
    }
}

impl ByteSize for crate::bitvec::sparse::SparseBitVec {
    fn byte_size(&self) -> u64 {
        mem::size_of_val(self) as u64
    }
}

impl ByteSize for crate::bitvec::rle::RLEBitVec {
    fn byte_size(&self) -> u64 {
        mem::size_of_val(self) as u64
    }
}

impl<T: crate::bitvec::BitVec + ByteSize> ByteSize for crate::bitvec::multi::Multi<T> {
    fn byte_size(&self) -> u64 {
        mem::size_of_val(self) as u64
    }
}

impl<T: crate::bitvec::BitVec + ByteSize> ByteSize for crate::bitvec::zeropadded::ZeroPadded<T> {
    fn byte_size(&self) -> u64 {
        mem::size_of_val(self) as u64
    }
}

impl<T: crate::bitvec::BitVec + ByteSize> ByteSize for crate::bitvec::onepadded::OnePadded<T> {
    fn byte_size(&self) -> u64 {
        // We have an accessor for the inner; subtract the inner's inline (which is
        // already counted in size_of_val(self)) so the inner's heap is added without
        // double-counting.
        mem::size_of_val(self) as u64 + self.inner().byte_size() - mem::size_of::<T>() as u64
    }
}

impl<BV: crate::bitvec::BitVec + ByteSize> ByteSize for crate::waveletmatrix::WaveletMatrix<BV> {
    fn byte_size(&self) -> u64 {
        let inline = mem::size_of_val(self) as u64;
        // Level<BV> is roughly BV + 3 u32s; we don't have an accessor for individual
        // levels but can approximate.
        let level_size = mem::size_of::<BV>() as u64 + 3 * mem::size_of::<u32>() as u64;
        inline + self.num_levels() as u64 * level_size
    }
}

impl ByteSize for crate::huffmanwaveletmatrix::HuffmanWaveletMatrix {
    fn byte_size(&self) -> u64 {
        // We can't reach the private symbol/code maps directly; approximate by
        // num_symbols.
        let inline = mem::size_of_val(self) as u64;
        let one_map = self.num_symbols() as u64
            * (mem::size_of::<u32>() as u64 * 2 + 16);
        inline + 2 * one_map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec::array::ArrayBitVecBuilder;
    use crate::bitvec::dense::DenseBitVecBuilder;
    use crate::bitvec::BitVecBuilder;
    use crate::bitvec::BitVecBuilderOf;
    use crate::bitvec::MultiBitVecBuilder;

    #[test]
    fn at_least_inline_size() {
        let bv = BitVecBuilderOf::<ArrayBitVecBuilder>::from_ones(100, Default::default(), &[1, 5, 10]);
        assert!(bv.inner().byte_size() >= mem::size_of_val(bv.inner()) as u64);

        let dense = DenseBitVecBuilder::from_ones(1024, Default::default(), &[10, 20, 30, 100]);
        assert!(dense.byte_size() >= mem::size_of_val(&dense) as u64);
    }

    #[test]
    fn array_bitvec_grows_with_ones() {
        let empty = BitVecBuilderOf::<ArrayBitVecBuilder>::from_ones(0, Default::default(), &[]);
        let big = BitVecBuilderOf::<ArrayBitVecBuilder>::from_ones(
            1000,
            Default::default(),
            &(0..100).collect::<Vec<_>>(),
        );
        assert!(big.inner().byte_size() >= empty.inner().byte_size() + 100 * 4);
    }
}
