//! `SymbolSequence` — the common query contract shared by `WaveletMatrix` and
//! `HuffmanWaveletMatrix`. Property tests against this trait validate both
//! implementations with one body of code.
//!
//! The trait is intentionally a *subset* of `WaveletMatrix`'s public API — the methods
//! both structures support and that are meaningful to cross-validate. Implementations
//! delegate to the underlying structure's methods.

use std::ops::Range;

pub trait SymbolSequence {
    fn get(&self, index: u32) -> u32;
    fn count(&self, range: Range<u32>, symbol: u32) -> u32;
    fn select(
        &self,
        range: Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32>;
    fn select_last(
        &self,
        range: Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32>;
    fn len(&self) -> u32;
    fn max_symbol(&self) -> u32;
}

impl<BV: crate::bitvec::BitVec> SymbolSequence for crate::waveletmatrix::WaveletMatrix<BV> {
    fn get(&self, index: u32) -> u32 {
        self.get(index)
    }
    fn count(&self, range: Range<u32>, symbol: u32) -> u32 {
        self.count(range, symbol)
    }
    fn select(
        &self,
        range: Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32> {
        self.select(range, symbol, k, ignore_bits)
    }
    fn select_last(
        &self,
        range: Range<u32>,
        symbol: u32,
        k: u32,
        ignore_bits: usize,
    ) -> Option<u32> {
        self.select_last(range, symbol, k, ignore_bits)
    }
    fn len(&self) -> u32 {
        self.len()
    }
    fn max_symbol(&self) -> u32 {
        self.max_symbol()
    }
}

// =========================================================================
// Property test helpers — exercise any `SymbolSequence` impl against naive
// references that operate directly on the source data. Used in both the
// `WaveletMatrix` and `HuffmanWaveletMatrix` test modules.
// =========================================================================

#[cfg(test)]
pub(crate) mod props {
    use super::SymbolSequence;
    use crate::test_support::{naive_count, naive_get, naive_select, naive_select_last};

    /// For every index `i`, `ss.get(i) == data[i]`.
    pub(crate) fn prop_get_vs_data<S: SymbolSequence>(data: &[u32], ss: &S) {
        for i in 0..data.len() as u32 {
            assert_eq!(ss.get(i), naive_get(data, i), "get({i})");
        }
    }

    /// For arbitrary ranges and symbols, `ss.count == naive_count`.
    pub(crate) fn prop_count_vs_naive<S: SymbolSequence>(
        data: &[u32],
        ss: &S,
        ranges: &[std::ops::Range<u32>],
        symbols: &[u32],
    ) {
        for range in ranges {
            for &sym in symbols {
                assert_eq!(
                    ss.count(range.clone(), sym),
                    naive_count(data, range.clone(), sym),
                    "count({range:?}, sym={sym})",
                );
            }
        }
    }

    /// `select(range, sym, k)` returns the index of the `k`-th `sym` in `range`,
    /// and `get` at that index equals `sym`. Returns `None` past the count.
    pub(crate) fn prop_select_get_roundtrip<S: SymbolSequence>(
        data: &[u32],
        ss: &S,
        range: std::ops::Range<u32>,
        symbol: u32,
    ) {
        let count = naive_count(data, range.clone(), symbol);
        for k in 0..count {
            let pos = ss
                .select(range.clone(), symbol, k, 0)
                .unwrap_or_else(|| panic!("select returned None at k={k}, range={range:?}"));
            assert!(range.contains(&pos), "select returned out-of-range pos={pos}");
            assert_eq!(ss.get(pos), symbol, "get(select({k})) != symbol");
            assert_eq!(naive_select(data, range.clone(), symbol, k), Some(pos));
        }
        // One past the last occurrence is None.
        assert_eq!(ss.select(range.clone(), symbol, count, 0), None);
    }

    /// `select_last(range, sym, k)` walks from the right; the `k`-th-from-end is the
    /// `(count-1-k)`-th-from-start.
    pub(crate) fn prop_select_last_consistency<S: SymbolSequence>(
        data: &[u32],
        ss: &S,
        range: std::ops::Range<u32>,
        symbol: u32,
    ) {
        let count = naive_count(data, range.clone(), symbol);
        for k in 0..count {
            let from_back = ss.select_last(range.clone(), symbol, k, 0);
            let from_front = ss.select(range.clone(), symbol, count - 1 - k, 0);
            assert_eq!(
                from_back, from_front,
                "select_last({k}) vs select({}) disagree",
                count - 1 - k
            );
            assert_eq!(from_back, naive_select_last(data, range.clone(), symbol, k));
        }
        assert_eq!(ss.select_last(range.clone(), symbol, count, 0), None);
    }

    /// Sum of `count(range, s)` over every distinct symbol in `data[range]` equals
    /// the range length. (Easy to break if a level's traversal logic mis-attributes
    /// elements to the wrong subtree.)
    pub(crate) fn prop_count_all_symbols_sums_to_range_length<S: SymbolSequence>(
        data: &[u32],
        ss: &S,
        range: std::ops::Range<u32>,
    ) {
        let mut distinct: Vec<u32> =
            data[range.start as usize..range.end as usize].to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        let total: u32 = distinct.iter().map(|&s| ss.count(range.clone(), s)).sum();
        assert_eq!(
            total,
            range.end - range.start,
            "sum of per-symbol counts != range length"
        );
    }
}

// =========================================================================
// Property suite parameterized over any `SymbolSequence` builder. Drives both the
// WaveletMatrix and HuffmanWaveletMatrix test modules.
// =========================================================================

#[cfg(test)]
pub(crate) fn run_symbol_sequence_props<S, F>(make: F)
where
    S: SymbolSequence,
    F: Fn(&[u32]) -> S,
{
    use arbtest::arbtest;

    arbtest(|u| {
        // Bound size and alphabet so tests stay snappy.
        let len = u.int_in_range(1u32..=64)?;
        let max_symbol = u.int_in_range(1u32..=8)?;
        let mut data: Vec<u32> = Vec::with_capacity(len as usize);
        for _ in 0..len {
            data.push(u.int_in_range(0u32..=max_symbol)?);
        }
        let ss = make(&data);

        props::prop_get_vs_data(&data, &ss);

        // Generate a handful of random ranges and run the range-based properties on each.
        let n = data.len() as u32;
        for _ in 0..4 {
            let a = u.int_in_range(0u32..=n)?;
            let b = u.int_in_range(0u32..=n)?;
            let range = a.min(b)..a.max(b);
            if range.start == range.end {
                continue;
            }

            // Test every symbol in 0..=ss.max_symbol() (respecting the WM contract,
            // which currently panics on out-of-range symbols rather than returning 0).
            let symbols: Vec<u32> = (0..=ss.max_symbol()).collect();
            props::prop_count_vs_naive(&data, &ss, &[range.clone()], &symbols);
            for &sym in &symbols {
                props::prop_select_get_roundtrip(&data, &ss, range.clone(), sym);
                props::prop_select_last_consistency(&data, &ss, range.clone(), sym);
            }
            props::prop_count_all_symbols_sums_to_range_length(&data, &ss, range.clone());
        }
        Ok(())
    });
}
