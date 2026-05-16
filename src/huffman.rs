//! Huffman code construction. Three free functions:
//!
//! - [`huffman_code_lengths`] — Moffat 2019's in-place linear-time length-only algorithm.
//! - [`canonical_huffman_codes`] — standard canonical codes (lex-ordered, non-decreasing length).
//! - [`wavelet_matrix_codes`] — wavelet-matrix-flavoured codes (short codes sort to the END of
//!   each level via bit-reversed ordering, so one-padding doesn't introduce gaps).
//!
//! Ported from `madeofbits/huffman.go`.

/// Generate canonical codewords from a non-decreasing array of Huffman code lengths.
/// Each returned code is right-aligned to its code length; viewed as bit-prefixes of
/// length `max_len`, consecutive codes are strictly increasing (the "lexicographic"
/// property), but the raw u32 values are not necessarily monotone across different
/// code lengths.
///
/// See Section 2.5 of Alistair Moffat, "Huffman Coding" (2019).
pub fn canonical_huffman_codes(lengths: &[u32]) -> Vec<u32> {
    let n = lengths.len();
    if n == 0 {
        return Vec::new();
    }
    let max_len = lengths[n - 1];
    let mut codes = vec![0u32; n];
    let mut code: u32 = 0;
    for i in 1..n {
        code += 1u32 << (max_len - lengths[i - 1]);
        codes[i] = code;
    }
    for i in 1..n {
        codes[i] >>= max_len - lengths[i];
    }
    codes
}

/// Generate Huffman-like optimal prefix-free codes optimized for a wavelet matrix.
///
/// Short codes are positioned such that they sort to the END of their lowest level in the
/// wavelet matrix when one-padded with trailing 1-bits. This means the "gaps" they introduce
/// into the full binary tree never appear between longer codes at deeper levels — every short
/// code is to the right of every longer code that survives it.
///
/// `lengths` must be sorted in ascending order (the natural output of [`huffman_code_lengths`]).
///
/// Ported from `WaveletMatrixCodes` in `madeofbits/huffman.go`.
pub fn wavelet_matrix_codes(lengths: &[u32]) -> Vec<u32> {
    if lengths.is_empty() {
        return Vec::new();
    }
    let max_len = lengths[lengths.len() - 1];

    // Histogram of code lengths.
    let mut hist = vec![0u32; (max_len + 1) as usize];
    for &l in lengths {
        hist[l as usize] += 1;
    }

    let mut codes: Vec<u32> = Vec::with_capacity(lengths.len());
    let mut current: Vec<u32> = vec![0, 1];
    for level in 1..=max_len {
        // current[split..] are the codes emitted at this level, in ascending bit-reversed order.
        let split = current.len() as u32 - hist[level as usize];
        codes.extend_from_slice(&current[(split as usize)..]);
        current.truncate(split as usize);

        if level == max_len {
            break;
        }
        // For each remaining code in `current`, append a 0-bit (in-place) and push the
        // 1-bit variant. The append-1 codes all sort after the append-0 codes, so the
        // overall slice remains sorted in ascending bit-reversed order.
        for i in 0..(split as usize) {
            current[i] *= 2;
            let with_one = current[i] + 1;
            current.push(with_one);
        }
    }
    codes
}

/// Compute Huffman codeword lengths via Moffat's linear-time Algorithm 2 (2019).
/// The algorithm runs in-place over an internal working array; `weights` is not modified.
///
/// `weights` must be sorted in **descending** weight order. Higher-weight symbols get
/// shorter codes. The returned lengths are sorted in ascending order (`lengths[0]` is the
/// shortest, `lengths[n-1]` is the longest).
///
/// Edge cases:
/// - `weights.is_empty()` → returns an empty `Vec`.
/// - Single-symbol input → returns `[0]` (zero-length code).
///
/// Ported from `HuffmanCodeLengths` in `madeofbits/huffman.go`. Variable names match the
/// Go source so the algorithm can be cross-referenced against §2.4 of the Moffat article.
pub fn huffman_code_lengths(weights: &[u32]) -> Vec<u32> {
    if weights.is_empty() {
        return Vec::new();
    }
    if weights.len() == 1 {
        return vec![0];
    }

    // Working array — modified in place through three phases.
    let mut w: Vec<u32> = weights.to_vec();
    let n = w.len();

    // Phase 1: combine pairs of smallest weights, recording parents in-place.
    let mut leaf: isize = (n - 1) as isize;
    let mut root: isize = (n - 1) as isize;
    let mut next: isize = (n - 1) as isize;
    while next > 0 {
        // First child.
        if leaf < 0 || (root > next && w[root as usize] < w[leaf as usize]) {
            w[next as usize] = w[root as usize];
            w[root as usize] = next as u32;
            root -= 1;
        } else {
            w[next as usize] = w[leaf as usize];
            leaf -= 1;
        }
        // Second child (add to next, don't replace).
        if leaf < 0 || (root > next && w[root as usize] < w[leaf as usize]) {
            w[next as usize] += w[root as usize];
            w[root as usize] = next as u32;
            root -= 1;
        } else {
            w[next as usize] += w[leaf as usize];
            leaf -= 1;
        }
        next -= 1;
    }

    // Phase 2: compute depths of internal nodes.
    w[1] = 0;
    for i in 2..n {
        w[i] = w[w[i] as usize] + 1;
    }

    // Phase 3: assign leaf depths by walking down the tree.
    let mut avail: i64 = 1;
    let mut used: i64 = 0;
    let mut depth: u32 = 0;
    let mut next_idx: usize = 0;
    let mut root_idx: usize = 1;
    while avail > 0 {
        while root_idx < n && w[root_idx] == depth {
            used += 1;
            root_idx += 1;
        }
        while avail > used {
            w[next_idx] = depth;
            next_idx += 1;
            avail -= 1;
        }
        avail = 2 * used;
        depth += 1;
        used = 0;
    }

    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn empty_inputs() {
        assert_eq!(huffman_code_lengths(&[]), Vec::<u32>::new());
        assert_eq!(canonical_huffman_codes(&[]), Vec::<u32>::new());
        assert_eq!(wavelet_matrix_codes(&[]), Vec::<u32>::new());
    }

    #[test]
    fn single_symbol() {
        assert_eq!(huffman_code_lengths(&[42]), vec![0]);
    }

    /// Kraft equality: for an optimal prefix code, sum(2^{-l_i}) == 1.
    /// Holds for every valid Huffman code-length set; arbtest sweep.
    #[test]
    fn prop_kraft_equality() {
        use arbtest::arbtest;
        arbtest(|u| {
            // 2..=16 distinct symbols with arbitrary positive weights, sorted descending.
            let n = u.int_in_range(2u32..=16)?;
            let mut weights: Vec<u32> = (0..n).map(|_| u.int_in_range(1u32..=1000)).collect::<Result<_, _>>()?;
            weights.sort_unstable_by(|a, b| b.cmp(a));
            let lengths = huffman_code_lengths(&weights);
            // Kraft sum in fixed-point — multiply through by 2^max_len.
            let max_len = *lengths.iter().max().unwrap();
            let kraft_sum: u64 = lengths.iter().map(|&l| 1u64 << (max_len - l)).sum();
            assert_eq!(kraft_sum, 1u64 << max_len, "Kraft sum != 1 for weights {weights:?}, lengths {lengths:?}");
            // Lengths are non-decreasing.
            for w in lengths.windows(2) {
                assert!(w[0] <= w[1], "lengths not non-decreasing: {lengths:?}");
            }
            Ok(())
        });
    }

    /// Snapshot the full pipeline (weights → lengths → canonical codes → WM codes) on a
    /// hand-picked weight set. Catches representation drift in the porting effort.
    #[test]
    fn snapshot_pipeline_small() {
        // Weights: 8, 4, 2, 1 (descending). Powers of 2 give a deterministic balanced tree.
        let weights = vec![8u32, 4, 2, 1];
        let lengths = huffman_code_lengths(&weights);
        let canonical = canonical_huffman_codes(&lengths);
        let wm = wavelet_matrix_codes(&lengths);
        let snap = format!(
            "weights={weights:?}\nlengths={lengths:?}\ncanonical={canonical:?}\nwm_codes={wm:?}"
        );
        expect![[r#"
            weights=[8, 4, 2, 1]
            lengths=[1, 2, 3, 3]
            canonical=[0, 2, 6, 7]
            wm_codes=[1, 1, 0, 1]"#]]
        .assert_eq(&snap);
    }

    /// `wavelet_matrix_codes` should be a prefix-free code (no code is a prefix of another)
    /// when each code is read at its declared length.
    #[test]
    fn prop_wm_codes_prefix_free() {
        use arbtest::arbtest;
        arbtest(|u| {
            let n = u.int_in_range(2u32..=12)?;
            let mut weights: Vec<u32> = (0..n).map(|_| u.int_in_range(1u32..=100)).collect::<Result<_, _>>()?;
            weights.sort_unstable_by(|a, b| b.cmp(a));
            let lengths = huffman_code_lengths(&weights);
            let codes = wavelet_matrix_codes(&lengths);

            // Pair each (code, length). Then assert no code is a prefix of any other at its length.
            for i in 0..codes.len() {
                for j in 0..codes.len() {
                    if i == j {
                        continue;
                    }
                    let (li, lj) = (lengths[i], lengths[j]);
                    if li > lj {
                        continue;
                    }
                    // Is codes[j]'s top li bits == codes[i]?
                    let prefix_of_j = codes[j] >> (lj - li);
                    if prefix_of_j == codes[i] {
                        panic!(
                            "WM codes not prefix-free: code {} (len {li}) is a prefix of code {} (len {lj}); weights={weights:?}",
                            codes[i], codes[j]
                        );
                    }
                }
            }
            Ok(())
        });
    }
}
