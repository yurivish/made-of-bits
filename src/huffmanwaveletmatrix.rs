//! Huffman-shaped wavelet matrix. Variable-length Huffman codes per symbol; shorter codes
//! sort to the end of each level via [`OnePadded`] wrapping, compressing the structure
//! towards first-order entropy.
//!
//! Wraps an inner [`WaveletMatrix<OnePadded<DenseBitVec>>`]. Each level is built from only
//! the data elements whose code length exceeds that level; trailing elements (with shorter
//! codes) are represented implicitly via one-padding.
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
    /// `symbol_to_code[sym]` → the 1-padded Huffman code for `sym`. Only present for
    /// symbols that actually appeared in the input.
    symbol_to_code: HashMap<u32, u32>,
    /// `code_to_symbol[padded_code]` → the original symbol. Map (not flat array) to avoid
    /// `2^max_code_len`-sized storage on skewed distributions where one rare symbol drags
    /// the max code length up.
    code_to_symbol: HashMap<u32, u32>,
    max_code_len: u32,
}

impl HuffmanWaveletMatrix {
    pub fn new(data: &[u32], bitvec_options: DenseBitVecOptions) -> Self {
        let n = data.len();
        assert!(n <= u32::MAX as usize, "data length must not exceed 2^32 - 1");

        if n == 0 {
            return Self {
                inner: empty_inner(),
                length: 0,
                max_symbol: 0,
                symbol_to_code: HashMap::new(),
                code_to_symbol: HashMap::new(),
                max_code_len: 0,
            };
        }

        // Count frequencies, find max symbol.
        let max_symbol = data.iter().copied().max().unwrap();
        let mut freqs: Vec<u32> = vec![0; (max_symbol + 1) as usize];
        for &d in data {
            freqs[d as usize] += 1;
        }

        // Build (weight, symbol) pairs for symbols that actually appear; sort by descending
        // weight then ascending symbol for deterministic tie-breaking (matches Go).
        let mut pairs: Vec<(u32, u32)> = freqs
            .iter()
            .enumerate()
            .filter(|(_, &w)| w > 0)
            .map(|(s, &w)| (w, s as u32))
            .collect();
        pairs.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

        let num_distinct = pairs.len();

        // Single-symbol corner case: no levels, no actual encoding.
        if num_distinct == 1 {
            let sym = pairs[0].1;
            let mut sym_to_code = HashMap::new();
            sym_to_code.insert(sym, 0u32);
            let mut code_to_sym = HashMap::new();
            code_to_sym.insert(0u32, sym);
            return Self {
                inner: empty_inner(),
                length: n as u32,
                max_symbol,
                symbol_to_code: sym_to_code,
                code_to_symbol: code_to_sym,
                max_code_len: 0,
            };
        }

        let weights: Vec<u32> = pairs.iter().map(|p| p.0).collect();
        let lengths = huffman_code_lengths(&weights);
        let codes = wavelet_matrix_codes(&lengths);
        let max_code_len = *lengths.last().unwrap();

        // Pad each code with trailing 1-bits to max_code_len: short codes occupy the END
        // of each level via one-padding, which is the property [`OnePadded`] exploits.
        let mut symbol_to_code: HashMap<u32, u32> = HashMap::with_capacity(num_distinct);
        let mut code_to_symbol: HashMap<u32, u32> = HashMap::with_capacity(num_distinct);
        let mut max_padded_code: u32 = 0;
        for (i, &(_, sym)) in pairs.iter().enumerate() {
            let code = codes[i];
            let code_len = lengths[i];
            let shift = max_code_len - code_len;
            let padded = (code << shift) | ((1u32 << shift) - 1);
            symbol_to_code.insert(sym, padded);
            code_to_symbol.insert(padded, sym);
            if padded > max_padded_code {
                max_padded_code = padded;
            }
        }

        // level_lens[l] = number of data elements whose code length is > l. (These are the
        // elements that still "exist" at level l before being short-circuited by the
        // OnePadded padding region.)
        let mut level_lens: Vec<u32> = vec![0; max_code_len as usize];
        for (i, &(weight, _)) in pairs.iter().enumerate() {
            for level in 0..lengths[i] {
                level_lens[level as usize] += weight;
            }
        }

        // Translate data into padded codes.
        let padded_data: Vec<u32> = data.iter().map(|&d| symbol_to_code[&d]).collect();

        // Build the level bitvecs. Each level processes only its `level_lens[l]` prefix; the
        // suffix is wrapped in OnePadded so all levels have universe_size == n externally.
        let inner_levels = build_huffman_bitvecs(
            &padded_data,
            max_code_len as usize,
            &level_lens,
            bitvec_options,
        );

        let inner = WaveletMatrix::<OnePadded<DenseBitVec>>::from_bitvecs(
            inner_levels,
            max_padded_code,
            None,
        );

        Self {
            inner,
            length: n as u32,
            max_symbol,
            symbol_to_code,
            code_to_symbol,
            max_code_len,
        }
    }

    /// Symbol at index `i`.
    pub fn get(&self, index: u32) -> u32 {
        assert!(index < self.length, "index out of bounds");
        if self.max_code_len == 0 {
            // Single-symbol case.
            return self.code_to_symbol[&0];
        }
        let code = self.inner.get(index);
        self.code_to_symbol[&code]
    }

    pub fn count(&self, range: std::ops::Range<u32>, symbol: u32) -> u32 {
        if self.length == 0 || range.start >= range.end {
            return 0;
        }
        if self.max_code_len == 0 {
            return if self.code_to_symbol[&0] == symbol {
                range.end - range.start
            } else {
                0
            };
        }
        let Some(&code) = self.symbol_to_code.get(&symbol) else {
            return 0;
        };
        self.inner.count(range, code)
    }

    /// `ignore_bits` must be 0; the contract exists only to mirror the
    /// SymbolSequence trait. Variable-length codes don't have a meaningful concept of
    /// "bottom k bits to ignore" — the bottom bits are padding for some symbols and
    /// data for others.
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
            return if self.code_to_symbol[&0] == symbol && k < range.end - range.start {
                Some(range.start + k)
            } else {
                None
            };
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
            return if self.code_to_symbol[&0] == symbol && k < range.end - range.start {
                Some(range.end - 1 - k)
            } else {
                None
            };
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
        let length = range.end - range.start;
        let half_len = length >> 1;
        let (code, count) = self.inner.quantile(range, half_len);
        if count > half_len {
            Some(self.code_to_symbol[&code])
        } else {
            None
        }
    }

    pub fn len(&self) -> u32 {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn max_symbol(&self) -> u32 {
        self.max_symbol
    }

    pub fn num_levels(&self) -> u32 {
        self.max_code_len
    }

    pub fn num_symbols(&self) -> u32 {
        self.code_to_symbol.len() as u32
    }
}

/// Sentinel empty inner WM. Used for empty / single-symbol HuffmanWMs that bypass the
/// actual wavelet-matrix machinery.
fn empty_inner() -> Inner {
    WaveletMatrix::<OnePadded<DenseBitVec>>::from_bitvecs(Vec::new(), 0, None)
}

/// Stable-partition Huffman bitvec builder.
///
/// At each level `l`, processes only the first `level_lens[l]` elements of `padded_data`
/// (which have code length > l). The remaining elements are implicit 1-bits handled by
/// the [`OnePadded`] wrapper around the resulting bitvec.
///
/// Mirrors `buildBitvecsPartition(..., huffmanLevelLens)` in `madeofbits/waveletmatrix.go`.
fn build_huffman_bitvecs(
    padded_data: &[u32],
    num_levels: usize,
    level_lens: &[u32],
    bitvec_options: DenseBitVecOptions,
) -> Vec<OnePadded<DenseBitVec>> {
    let n = padded_data.len() as u32;
    let mut data: Vec<u32> = padded_data.to_vec();
    let mut result: Vec<OnePadded<DenseBitVec>> = Vec::with_capacity(num_levels);
    let max_level = num_levels - 1;
    let mut right: Vec<u32> = Vec::new();

    for l in 0..num_levels {
        let active_len = level_lens[l];
        let level_bit = 1u32 << (max_level - l);

        let mut b = DenseBitVecBuilder::new(active_len, bitvec_options);

        // Stable partition the first `active_len` elements: 0-bits stay in place,
        // 1-bits move to `right` and are appended at the end.
        right.clear();
        let mut write_idx = 0usize;
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
        // Copy the right partition back into the slot just after the left partition.
        for (j, &v) in right.iter().enumerate() {
            data[write_idx + j] = v;
        }
        let bv = b.build();
        // Always wrap in OnePadded, even when active_len == n, so the inner WM sees a
        // homogeneous level type. Zero overhead when inner_len == universe.
        result.push(OnePadded::new(bv, n));
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
