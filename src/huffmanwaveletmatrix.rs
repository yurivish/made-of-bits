//! Huffman-shaped wavelet matrix. Variable-length Huffman codes per symbol; shorter
//! codes sort to the end of each level via [`OnePadded`] wrapping, compressing the
//! structure towards first-order entropy.
//!
//! Ported from `madeofbits/huffmanwaveletmatrix.go`.

use std::collections::HashMap;

use crate::bitvec::dense::{DenseBitVec, DenseBitVecBuilder, DenseBitVecOptions};
use crate::bitvec::onepadded::OnePadded;
use crate::bitvec::BitVec;
use crate::bitvec::BitVecBuilder;
use crate::huffman::{huffman_code_lengths, wavelet_matrix_codes};
use crate::waveletmatrix::WaveletMatrix;

type Inner = WaveletMatrix<OnePadded<DenseBitVec>>;

pub struct HuffmanWaveletMatrix {
    inner: Inner,
    length: u32,
    max_symbol: u32,
    // Map (not flat 2^max_code_len array) because a single rare symbol can drag the
    // max code length up arbitrarily; only num_distinct entries get stored.
    symbol_to_code: HashMap<u32, u32>,
    code_to_symbol: HashMap<u32, u32>,
    max_code_len: u32,
}

impl HuffmanWaveletMatrix {
    pub fn new(data: &[u32], bitvec_options: DenseBitVecOptions) -> Self {
        let n = data.len();
        assert!(n <= u32::MAX as usize, "data length must not exceed 2^32 - 1");

        if n == 0 {
            return Self::degenerate(0, 0, HashMap::new(), HashMap::new());
        }

        let max_symbol = data.iter().copied().max().unwrap();
        let mut freqs = vec![0u32; (max_symbol + 1) as usize];
        for &d in data {
            freqs[d as usize] += 1;
        }

        // (weight, symbol) pairs for symbols that appear, sorted by descending weight,
        // ascending symbol for tie-breaking (matches Go).
        let mut pairs: Vec<(u32, u32)> = freqs
            .iter()
            .enumerate()
            .filter(|(_, &w)| w > 0)
            .map(|(s, &w)| (w, s as u32))
            .collect();
        pairs.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

        // Single-symbol case bypasses Huffman entirely: code length 0, no levels.
        if pairs.len() == 1 {
            let sym = pairs[0].1;
            return Self::degenerate(
                n as u32,
                max_symbol,
                HashMap::from([(sym, 0)]),
                HashMap::from([(0, sym)]),
            );
        }

        let weights: Vec<u32> = pairs.iter().map(|p| p.0).collect();
        let lengths = huffman_code_lengths(&weights);
        let codes = wavelet_matrix_codes(&lengths);
        let max_code_len = *lengths.last().unwrap();

        // Pad each code with trailing 1-bits to max_code_len so short codes occupy the
        // END of each level — the property OnePadded exploits.
        let mut symbol_to_code = HashMap::with_capacity(pairs.len());
        let mut code_to_symbol = HashMap::with_capacity(pairs.len());
        let mut max_padded_code = 0u32;
        for (i, &(_, sym)) in pairs.iter().enumerate() {
            let shift = max_code_len - lengths[i];
            let padded = (codes[i] << shift) | ((1u32 << shift) - 1);
            symbol_to_code.insert(sym, padded);
            code_to_symbol.insert(padded, sym);
            max_padded_code = max_padded_code.max(padded);
        }

        // level_lens[l] = number of data elements with code length > l (the elements
        // still "alive" at level l, before being absorbed by OnePadded's padding).
        let mut level_lens = vec![0u32; max_code_len as usize];
        for (i, &(weight, _)) in pairs.iter().enumerate() {
            for level in 0..lengths[i] {
                level_lens[level as usize] += weight;
            }
        }

        let padded_data: Vec<u32> = data.iter().map(|&d| symbol_to_code[&d]).collect();
        let inner_levels =
            build_huffman_bitvecs(&padded_data, &level_lens, bitvec_options);
        let inner = WaveletMatrix::from_bitvecs(inner_levels, max_padded_code, None);

        Self {
            inner,
            length: n as u32,
            max_symbol,
            symbol_to_code,
            code_to_symbol,
            max_code_len,
        }
    }

    fn degenerate(
        length: u32,
        max_symbol: u32,
        symbol_to_code: HashMap<u32, u32>,
        code_to_symbol: HashMap<u32, u32>,
    ) -> Self {
        Self {
            inner: WaveletMatrix::from_bitvecs(Vec::new(), 0, None),
            length,
            max_symbol,
            symbol_to_code,
            code_to_symbol,
            max_code_len: 0,
        }
    }

    pub fn get(&self, index: u32) -> u32 {
        assert!(index < self.length, "index out of bounds");
        if self.max_code_len == 0 {
            return self.code_to_symbol[&0];
        }
        self.code_to_symbol[&self.inner.get(index)]
    }

    pub fn count(&self, range: std::ops::Range<u32>, symbol: u32) -> u32 {
        if self.length == 0 || range.start >= range.end {
            return 0;
        }
        if self.max_code_len == 0 {
            return if self.code_to_symbol[&0] == symbol { range.end - range.start } else { 0 };
        }
        match self.symbol_to_code.get(&symbol) {
            Some(&code) => self.inner.count(range, code),
            None => 0,
        }
    }

    /// `ignore_bits` must be 0 (variable-length codes have no meaningful "bottom k bits
    /// to ignore"). Present only to mirror the SymbolSequence trait.
    pub fn select(
        &self,
        range: std::ops::Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32> {
        assert_eq!(ignore_bits, 0, "HuffmanWaveletMatrix.select: ignore_bits must be 0");
        if self.length == 0 || range.start >= range.end {
            return None;
        }
        if self.max_code_len == 0 {
            return (self.code_to_symbol[&0] == symbol && k < range.end - range.start)
                .then(|| range.start + k);
        }
        let &code = self.symbol_to_code.get(&symbol)?;
        self.inner.select(range, code, k, 0)
    }

    pub fn select_last(
        &self,
        range: std::ops::Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32> {
        assert_eq!(ignore_bits, 0, "HuffmanWaveletMatrix.select_last: ignore_bits must be 0");
        if self.length == 0 || range.start >= range.end {
            return None;
        }
        if self.max_code_len == 0 {
            return (self.code_to_symbol[&0] == symbol && k < range.end - range.start)
                .then(|| range.end - 1 - k);
        }
        let &code = self.symbol_to_code.get(&symbol)?;
        self.inner.select_last(range, code, k, 0)
    }

    pub fn simple_majority(&self, range: std::ops::Range<u32>) -> Option<u32> {
        if self.length == 0 || range.start >= range.end {
            return None;
        }
        if self.max_code_len == 0 {
            return Some(self.code_to_symbol[&0]);
        }
        let half_len = (range.end - range.start) >> 1;
        let (code, count) = self.inner.quantile(range, half_len);
        (count > half_len).then(|| self.code_to_symbol[&code])
    }

    pub fn len(&self) -> u32 { self.length }
    pub fn is_empty(&self) -> bool { self.length == 0 }

    /// The largest input symbol seen during construction (not the largest *encoded*
    /// symbol; padded Huffman codes can exceed this).
    pub fn max_symbol(&self) -> u32 { self.max_symbol }

    /// Number of levels in the underlying wavelet matrix, equal to the maximum Huffman
    /// code length. Zero for degenerate inputs (empty or single-symbol).
    pub fn num_levels(&self) -> u32 { self.max_code_len }

    /// Number of distinct symbols that appear in the input.
    pub fn num_symbols(&self) -> u32 { self.code_to_symbol.len() as u32 }
}

/// Stable-partition Huffman bitvec builder. Each level processes only the
/// `level_lens[l]` prefix (elements with code length > l); the rest is absorbed by the
/// surrounding OnePadded wrapper. Mirrors `buildBitvecsPartition(..., huffmanLevelLens)`
/// in `madeofbits/waveletmatrix.go`.
fn build_huffman_bitvecs(
    padded_data: &[u32],
    level_lens: &[u32],
    bitvec_options: DenseBitVecOptions,
) -> Vec<OnePadded<DenseBitVec>> {
    let n = padded_data.len() as u32;
    let num_levels = level_lens.len();
    let max_level = num_levels - 1;
    let mut data = padded_data.to_vec();
    let mut result = Vec::with_capacity(num_levels);
    let mut right = Vec::new();

    for l in 0..num_levels {
        let active_len = level_lens[l];
        let level_bit = 1u32 << (max_level - l);
        let mut b = DenseBitVecBuilder::new(active_len, bitvec_options);

        right.clear();
        let mut write_idx = 0;
        for i in 0..active_len as usize {
            let d = data[i];
            if d & level_bit == 0 {
                data[write_idx] = d;
                write_idx += 1;
            } else {
                b.one(i as u32);
                right.push(d);
            }
        }
        data[write_idx..write_idx + right.len()].copy_from_slice(&right);
        // Always wrap in OnePadded so the inner WM sees a homogeneous level type; zero
        // overhead when active_len == n.
        result.push(OnePadded::new(b.build(), n));
    }
    result
}

// ============================================================================
// SymbolSequence impl — cross-validates against WaveletMatrix in tests.
// ============================================================================

impl crate::symbol_sequence::SymbolSequence for HuffmanWaveletMatrix {
    fn get(&self, index: u32) -> u32 {
        self.get(index)
    }
    fn count(&self, range: std::ops::Range<u32>, symbol: u32) -> u32 {
        self.count(range, symbol)
    }
    fn select(
        &self,
        range: std::ops::Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32> {
        self.select(range, symbol, k, ignore_bits)
    }
    fn select_last(
        &self,
        range: std::ops::Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32> {
        self.select_last(range, symbol, k, ignore_bits)
    }
    fn len(&self) -> u32 {
        self.length
    }
    fn max_symbol(&self) -> u32 {
        self.max_symbol
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waveletmatrix::WaveletMatrix;

    /// Spot test on a tiny known dataset.
    #[test]
    fn spot_basic() {
        // data with skewed distribution: 'a' very frequent, others rare.
        let data: Vec<u32> = (0..20i32)
            .map(|i| match i % 5 {
                0 | 2 | 3 => 1u32, // common
                1 => 2,            // less common
                _ => 100,          // rare
            })
            .collect();
        let hwm = HuffmanWaveletMatrix::new(&data, Default::default());

        assert_eq!(hwm.len(), data.len() as u32);
        assert_eq!(hwm.max_symbol(), 100);
        assert!(hwm.num_levels() >= 1);

        // get round-trip
        for (i, &expected) in data.iter().enumerate() {
            assert_eq!(hwm.get(i as u32), expected, "get({i})");
        }
        // count vs naive
        for sym in [1u32, 2, 100, 99] {
            let expected = data.iter().filter(|&&x| x == sym).count() as u32;
            assert_eq!(hwm.count(0..data.len() as u32, sym), expected, "count({sym})");
        }
    }

    /// Empty input → empty HuffmanWM, queries return Nones/zeros.
    #[test]
    fn empty_input() {
        let hwm = HuffmanWaveletMatrix::new(&[], Default::default());
        assert_eq!(hwm.len(), 0);
        assert!(hwm.is_empty());
        assert_eq!(hwm.count(0..0, 42), 0);
        assert_eq!(hwm.select(0..0, 42, 0, 0), None);
    }

    /// Single-symbol input → degenerate case, max_code_len == 0.
    #[test]
    fn single_symbol_input() {
        let data = vec![7u32; 10];
        let hwm = HuffmanWaveletMatrix::new(&data, Default::default());
        assert_eq!(hwm.num_levels(), 0);
        assert_eq!(hwm.num_symbols(), 1);
        for i in 0..10 {
            assert_eq!(hwm.get(i), 7);
        }
        assert_eq!(hwm.count(0..10, 7), 10);
        assert_eq!(hwm.count(2..7, 7), 5);
        assert_eq!(hwm.count(0..10, 8), 0);
        assert_eq!(hwm.select(0..10, 7, 0, 0), Some(0));
        assert_eq!(hwm.select(0..10, 7, 9, 0), Some(9));
        assert_eq!(hwm.select(0..10, 7, 10, 0), None);
        assert_eq!(hwm.simple_majority(0..10), Some(7));
    }

    /// Property test: every overlapping query (get, count, select, select_last,
    /// simple_majority) must agree between WaveletMatrix and HuffmanWaveletMatrix
    /// on identical data. This is the central correctness property — HuffmanWM has no
    /// other independent reference.
    #[test]
    fn hwm_agrees_with_wm() {
        use arbtest::arbtest;
        arbtest(|u| {
            let len = u.int_in_range(1u32..=64)?;
            let max_sym = u.int_in_range(1u32..=8)?;
            let mut data: Vec<u32> = Vec::with_capacity(len as usize);
            for _ in 0..len {
                data.push(u.int_in_range(0u32..=max_sym)?);
            }
            let wm: WaveletMatrix<DenseBitVec> = WaveletMatrix::new(
                data.clone(),
                max_sym,
                Default::default(),
                None,
            );
            let hwm = HuffmanWaveletMatrix::new(&data, Default::default());

            // get
            for i in 0..len {
                assert_eq!(hwm.get(i), wm.get(i), "get({i})");
            }
            // count for every symbol in 0..=max_sym, full range.
            for sym in 0..=max_sym {
                assert_eq!(
                    hwm.count(0..len, sym),
                    wm.count(0..len, sym),
                    "count(0..{len}, {sym})",
                );
            }
            // A few random sub-ranges.
            for _ in 0..3 {
                let a = u.int_in_range(0u32..=len)?;
                let b = u.int_in_range(0u32..=len)?;
                let range = a.min(b)..a.max(b);
                if range.start == range.end {
                    continue;
                }
                for sym in 0..=max_sym {
                    assert_eq!(hwm.count(range.clone(), sym), wm.count(range.clone(), sym));
                    let count = wm.count(range.clone(), sym);
                    for k in 0..count {
                        assert_eq!(
                            hwm.select(range.clone(), sym, k, 0),
                            wm.select(range.clone(), sym, k, 0),
                            "select({range:?}, {sym}, {k})",
                        );
                        assert_eq!(
                            hwm.select_last(range.clone(), sym, k, 0),
                            wm.select_last(range.clone(), sym, k, 0),
                        );
                    }
                }
                assert_eq!(hwm.simple_majority(range.clone()), wm.simple_majority(range.clone()));
            }
            Ok(())
        });
    }

    /// Same SymbolSequence property suite as WaveletMatrix, applied to HuffmanWaveletMatrix.
    #[test]
    fn symbol_sequence_props() {
        crate::symbol_sequence::run_symbol_sequence_props(|data: &[u32]| {
            HuffmanWaveletMatrix::new(data, Default::default())
        });
    }
}
