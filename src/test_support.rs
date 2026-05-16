//! Naive reference implementations used by property tests as ground truth.
//!
//! Every method here is O(n) or O(n · m) — never the right thing to use in production,
//! but obvious enough to be trustworthy. Cross-validating the real implementations
//! against these is the spine of the test strategy.

#![cfg(test)]

use std::ops::Range;

/// `data[index]`, with bounds-check semantics matching `WaveletMatrix::get`
/// (panics if out of range).
pub(crate) fn naive_get(data: &[u32], index: u32) -> u32 {
    data[index as usize]
}

/// Number of times `symbol` appears in `data[range]`.
pub(crate) fn naive_count(data: &[u32], range: Range<u32>, symbol: u32) -> u32 {
    data[range.start as usize..range.end as usize]
        .iter()
        .filter(|&&x| x == symbol)
        .count() as u32
}

/// Position of the `k`-th (zero-indexed) occurrence of `symbol` in `data[range]`,
/// counting from the left. Returns `None` if fewer than `k+1` occurrences exist.
pub(crate) fn naive_select(
    data: &[u32],
    range: Range<u32>,
    symbol: u32,
    k: u32,
) -> Option<u32> {
    let mut count = 0u32;
    for i in range.start..range.end {
        if data[i as usize] == symbol {
            if count == k {
                return Some(i);
            }
            count += 1;
        }
    }
    None
}

/// Position of the `k`-th (zero-indexed) occurrence of `symbol` in `data[range]`,
/// counting from the right.
pub(crate) fn naive_select_last(
    data: &[u32],
    range: Range<u32>,
    symbol: u32,
    k: u32,
) -> Option<u32> {
    let mut count = 0u32;
    for i in (range.start..range.end).rev() {
        if data[i as usize] == symbol {
            if count == k {
                return Some(i);
            }
            count += 1;
        }
    }
    None
}

/// The `k`-th smallest symbol in `data[range]` (sort the slice, take element `k`),
/// together with the number of times it occurs in the range.
pub(crate) fn naive_quantile(data: &[u32], range: Range<u32>, k: u32) -> (u32, u32) {
    let mut sub: Vec<u32> = data[range.start as usize..range.end as usize].to_vec();
    sub.sort_unstable();
    let symbol = sub[k as usize];
    let count = sub.iter().filter(|&&x| x == symbol).count() as u32;
    (symbol, count)
}

/// Naive `preceding_count`: number of elements in `data[range]` strictly less than `symbol`.
pub(crate) fn naive_preceding_count(data: &[u32], range: Range<u32>, symbol: u32) -> u32 {
    data[range.start as usize..range.end as usize]
        .iter()
        .filter(|&&x| x < symbol)
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_helpers_basic() {
        let data = [1, 3, 3, 2, 7, 3];
        assert_eq!(naive_get(&data, 2), 3);
        assert_eq!(naive_count(&data, 0..data.len() as u32, 3), 3);
        // data[2..5] = [3, 2, 7] → 3 appears once.
        assert_eq!(naive_count(&data, 2..5, 3), 1);
        assert_eq!(naive_select(&data, 0..data.len() as u32, 3, 0), Some(1));
        assert_eq!(naive_select(&data, 0..data.len() as u32, 3, 2), Some(5));
        assert_eq!(naive_select(&data, 0..data.len() as u32, 3, 3), None);
        assert_eq!(naive_select_last(&data, 0..data.len() as u32, 3, 0), Some(5));
        assert_eq!(naive_select_last(&data, 0..data.len() as u32, 3, 2), Some(1));
        assert_eq!(naive_quantile(&data, 0..data.len() as u32, 0), (1, 1));
        assert_eq!(naive_quantile(&data, 0..data.len() as u32, 3), (3, 3));
        assert_eq!(naive_quantile(&data, 0..data.len() as u32, 5), (7, 1));
        assert_eq!(naive_preceding_count(&data, 0..data.len() as u32, 3), 2);
    }
}
