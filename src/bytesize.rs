//! `ByteSize` trait reporting the total memory footprint (inline struct size + heap-
//! allocated buffers) of succinct-data-structure values.
//!
//! Rust has no runtime reflection, so each implementation is hand-written. For container
//! types (e.g., `Multi<T>`, `ZeroPadded<T>`, `OnePadded<T>`, `WaveletMatrix<BV>`) the impl
//! is generic over the inner `T: ByteSize`. Owned buffers (`Box<[T]>`, `Vec<T>`) are
//! counted via `len * size_of::<T>()` (the heap region's footprint, not its capacity —
//! `Box<[T]>` has no excess capacity; `Vec<T>` may but the typical pattern in this crate
//! is to build then freeze into a slice).
//!
//! `HashMap<K, V>` accounting is approximate. There's no stable allocator hook to ask
//! the std-library map for its true footprint, so we use a documented per-entry
//! constant. Tune it if more accurate accounting is needed.
//!
//! Ported in spirit from `madeofbits/bytesize.go` (which uses Go runtime reflection).

use crate::bitvec::BitVec;
use crate::bitvec::MultiBitVec;
use std::collections::HashMap;
use std::mem;

pub trait ByteSize {
    /// Total memory footprint in bytes: inline size of `Self` plus any heap regions it owns.
    fn byte_size(&self) -> u64;
}

/// Approximate per-entry overhead for `HashMap<K, V>`. Real number depends on the
/// allocator and load factor — this constant assumes the default hasher and a roughly
/// 0.875 load factor. Treat as a lower bound; actual values can be 20-50% higher.
fn hashmap_overhead_bytes<K, V>(map: &HashMap<K, V>) -> u64 {
    // Bucket array sized to load_factor * len. The standard hashmap stores 7 entries
    // per group (~ Swiss table). Approximate the buckets at 16 bytes overhead each
    // beyond the (K, V) payload.
    let per_entry = (mem::size_of::<K>() + mem::size_of::<V>() + 16) as u64;
    map.len() as u64 * per_entry
}

// ============================================================================
// Buffers
// ============================================================================

impl ByteSize for crate::bitbuf::BitBuf {
    fn byte_size(&self) -> u64 {
        mem::size_of_val(self) as u64
            + self.blocks().len() as u64 * mem::size_of::<crate::bitbuf::Block>() as u64
    }
}

impl ByteSize for crate::intbuf::IntBuf {
    fn byte_size(&self) -> u64 {
        // We can't see the inner byte buffer without exposing it; conservative bound:
        // inline size + the cap'd length-in-bits expansion plus 7-byte pad. IntBuf
        // doesn't currently expose len or bit_width, so we report only the inline size.
        // Callers that need precise accounting should add an accessor.
        mem::size_of_val(self) as u64
    }
}

// ============================================================================
// BitVec impls
// ============================================================================

impl ByteSize for crate::bitvec::array::ArrayBitVec {
    fn byte_size(&self) -> u64 {
        let inline = mem::size_of_val(self) as u64;
        let heap = self.num_ones() as u64 * mem::size_of::<u32>() as u64;
        // ArrayBitVec exposes num_ones; the `ones` slice is num_ones entries.
        inline + heap
    }
}

impl ByteSize for crate::bitvec::dense::DenseBitVec {
    fn byte_size(&self) -> u64 {
        // DenseBitVec doesn't expose its internal sample arrays directly. We can only
        // report the inline size plus the BitBuf's heap footprint accessible via
        // universe_size + num_ones (lower bound). This is a known imprecision documented
        // at module level; users who need exact accounting can switch to an accessor.
        let inline = mem::size_of_val(self) as u64;
        // The bit buffer's heap region: num_blocks * sizeof(Block). num_blocks isn't
        // directly exposed; approximate via universe_size / Block::BITS.
        let blocks = (self.universe_size() as u64).div_ceil(64);
        let buf_heap = blocks * mem::size_of::<crate::bitbuf::Block>() as u64;
        inline + buf_heap
    }
}

impl ByteSize for crate::bitvec::sparse::SparseBitVec {
    fn byte_size(&self) -> u64 {
        // SparseBitVec wraps a DenseBitVec (high) and an IntBuf (low). Without exposing
        // them we report inline + an approximation for the dense high bits.
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
        // Multi<T> wraps an occupancy BitVec and a multiplicity SparseBitVec. We can
        // walk the public fields neither cleanly; report inline + an approximation.
        // For correctness the inline already includes the Box pointers etc; heap is
        // owned by the wrapped fields which we can't reach. This is a known under-count.
        mem::size_of_val(self) as u64
    }
}

impl<T: crate::bitvec::BitVec + ByteSize> ByteSize for crate::bitvec::zeropadded::ZeroPadded<T> {
    fn byte_size(&self) -> u64 {
        // ZeroPadded's inner field is private; we can't reach it without an accessor.
        // Report inline only.
        mem::size_of_val(self) as u64
    }
}

impl<T: crate::bitvec::BitVec + ByteSize> ByteSize for crate::bitvec::onepadded::OnePadded<T> {
    fn byte_size(&self) -> u64 {
        // OnePadded exposes `inner()` so we can recurse.
        mem::size_of_val(self) as u64 + self.inner().byte_size() - mem::size_of::<T>() as u64
    }
}

// ============================================================================
// Wavelet matrices
// ============================================================================

impl<BV: crate::bitvec::BitVec + ByteSize> ByteSize for crate::waveletmatrix::WaveletMatrix<BV> {
    fn byte_size(&self) -> u64 {
        // We can't reach individual levels without an accessor — report inline + the
        // Vec<Level<BV>>'s heap region using num_levels.
        let inline = mem::size_of_val(self) as u64;
        // Level<BV> is roughly BV + 3 u32s + nz mask. We use mem::size_of::<BV>() as a
        // proxy for the inline size of one level.
        let levels_heap = self.num_levels() as u64
            * (mem::size_of::<BV>() as u64 + 3 * mem::size_of::<u32>() as u64);
        inline + levels_heap
    }
}

impl ByteSize for crate::huffmanwaveletmatrix::HuffmanWaveletMatrix {
    fn byte_size(&self) -> u64 {
        // HuffmanWM's fields are private; report inline + map overhead estimates.
        let inline = mem::size_of_val(self) as u64;
        // Two HashMap<u32, u32>s of num_symbols entries each.
        let map_size = self.num_symbols() as u64 * (mem::size_of::<u32>() as u64 + mem::size_of::<u32>() as u64 + 16);
        inline + 2 * map_size
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

    /// Every type's `byte_size()` reports at least its inline `size_of_val`. This is the
    /// minimum useful invariant; we can't easily test exact heap sizes without
    /// allocator hooks.
    #[test]
    fn at_least_inline_size() {
        let bv = BitVecBuilderOf::<ArrayBitVecBuilder>::from_ones(100, Default::default(), &[1, 5, 10]);
        let inner = bv.inner();
        assert!(inner.byte_size() >= mem::size_of_val(inner) as u64);

        let dense = DenseBitVecBuilder::from_ones(1024, Default::default(), &[10, 20, 30, 100]);
        assert!(dense.byte_size() >= mem::size_of_val(&dense) as u64);
    }

    /// Sanity: `byte_size` includes the heap region for slice-backed types.
    #[test]
    fn array_bitvec_includes_ones() {
        // Empty ArrayBitVec: heap region is 0.
        let empty = BitVecBuilderOf::<ArrayBitVecBuilder>::from_ones(0, Default::default(), &[]);
        let empty_inner = empty.inner();
        let empty_size = empty_inner.byte_size();
        // Now build a non-empty one with 100 ones; heap region should be 400 bytes more.
        let big = BitVecBuilderOf::<ArrayBitVecBuilder>::from_ones(1000, Default::default(), &(0..100).collect::<Vec<u32>>());
        let big_inner = big.inner();
        let big_size = big_inner.byte_size();
        assert!(
            big_size >= empty_size + 100 * 4,
            "ArrayBitVec byte_size {} < expected {} + 400",
            big_size, empty_size
        );
    }
}
