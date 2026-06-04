# Made of Bits

This library is the product of my explorations into [succinct data structures](https://en.wikipedia.org/wiki/Succinct_data_structure) for data visualization. It implements bit vectors with [rank and select operations](https://en.wikipedia.org/wiki/Succinct_data_structure#Succinct_indexable_dictionaries), the [wavelet matrix](https://users.dcc.uchile.cl/~gnavarro/ps/spire12.4.pdf) generalising rank/select to larger alphabets, a Huffman-shaped variant of it for skewed data, and a few supporting pieces: Z-order primitives, Sequential Binary Interpolative Coding, and a memory-accounting trait.

## Bit vectors

Several bit vector types, each specialised to a particular data pattern:

- The [dense](src/bitvec/dense.rs) bit vector is based on the approach proposed in [Fast, Small, Simple Rank/Select on Bitmaps](https://www.dcc.uchile.cl/~gnavarro/ps/sea12.1.pdf). In-block selection uses [Vigna's broadword `select64`](https://vigna.di.unimi.it/ftp/papers/Broadword.pdf).
- The [sparse](src/bitvec/sparse.rs) bit vector is implemented using [Elias-Fano encoding](https://www.antoniomallia.it/sorted-integers-compression-with-elias-fano-encoding.html), with a static offset that shrinks the effective universe when the first 1-bit sits far from 0.
- The [run-length-encoded](src/bitvec/rle.rs) bit vector uses an idea I developed together with [Gonzalo Navarro](https://users.dcc.uchile.cl/~gnavarro/), which is a tweak to a RLE bit vector described in his [book](https://www.amazon.com/Compact-Data-Structures-Practical-Approach/dp/1107152380) to improve the efficiency of rank queries. We wrote a [technical note](https://archive.yuri.is/pdfing/weighted_range_quantile_queries.pdf) about it.
- The [array](src/bitvec/array.rs) bit vector is backed by a plain sorted array, and serves as a useful baseline for correctness testing.

In addition, there are wrapper types that grant new powers to an existing bit vector:

- [Multi\<T\>](src/bitvec/multi.rs) wraps a plain bit vector and turns it into a "multi bit vector" which stores counts and allows multiple repetitions of the same bit to be stored (if a bit vector is a set, a multi bit vector is a multiset).
- [ZeroPadded\<T\>](src/bitvec/zeropadded.rs) embeds a bit vector into a larger universe by padding it on the left and right with 0-bits. These extra bits take no space as they are represented implicitly.
- [OnePadded\<T\>](src/bitvec/onepadded.rs) is the mirror: implicit 1-bits on the right, with no storage cost. Used internally by the Huffman wavelet matrix to push shorter codes to the end of each level.

## Wavelet matrix

The crown jewel of this crate is the [wavelet matrix](src/waveletmatrix.rs) data structure, which generalizes rank/select bit vectors to larger integer alphabets than (_0_, _1_). (The [wavelet matrix](https://users.dcc.uchile.cl/~gnavarro/ps/spire12.4.pdf) is a variant of the [wavelet tree](https://www.sciencedirect.com/science/article/pii/S1570866713000610) that improves space efficiency and runtime performance, particularly for large alphabets.)

This crate provides wavelet matrix rank, select, [range quantile](https://arxiv.org/abs/0903.4726), `simple_majority`, range count, and `select_first_less_than`. It also provides several batch operations that descend the tree together rather than once per query: `quantile_batch` (many ks over one range), `majority(k)` (symbols with frequency > 1/k), `count_batch` (multiple symbol ranges over one index range), `locate_batch`, `counts`, and `counts_faster_maybe`.

Storing [Morton codes](https://en.wikipedia.org/wiki/Z-order_curve) in the wavelet matrix enables multi-dimensional range count queries (`morton_count_batch`), which is great fun for zoomable density scatterplots with adaptive level-of-detail. Supporting Z-order primitives — 2D and 3D encode/decode, [Tropf-Herzog](https://en.wikipedia.org/wiki/Z-order_curve) LITMAX/BIGMIN, generalised `dim_mask` — live in [src/zorder/mod.rs](src/zorder/mod.rs).

### Huffman-shaped wavelet matrix

[HuffmanWaveletMatrix](src/huffmanwaveletmatrix.rs) assigns variable-length Huffman codes to symbols, compressing the structure toward first-order entropy. Short codes are positioned (via [OnePadded\<T\>](src/bitvec/onepadded.rs)) so they sort to the end of each level — the "gaps" they introduce never appear between longer codes at deeper levels, which preserves the rank-based traversal. Supported queries: `get`, `count`, `select`, `select_last`, `simple_majority`.

The standalone [huffman](src/huffman.rs) module exposes the underlying code-construction routines (Moffat 2019's in-place linear-time length algorithm, canonical Huffman codes, and a wavelet-matrix-flavoured variant) for use on their own.

The [SymbolSequence](src/symbol_sequence.rs) trait abstracts over both `WaveletMatrix` and `HuffmanWaveletMatrix`, and a shared property-test suite cross-validates the two implementations against each other and against naive linear-scan references.

## Other modules

- [bic](src/bic.rs) — Sequential Binary Interpolative Coding (Algorithm 1 of [Moffat 2024](https://arxiv.org/abs/2401.05226)) for compressing strictly-positive integer sequences.
- [bytesize](src/bytesize.rs) — `ByteSize` trait reporting total memory footprint (inline + heap) per type.

## Traits

This crate defines several traits:

- [BitVec](src/bitvec/mod.rs) is implemented by plain bit vectors (which are functionally integer sets).
- [MultiBitVec](src/bitvec/mod.rs) is implemented by bit vector types that support storing the same integer multiple times. `ArrayBitVec` and `SparseBitVec` store repetitions explicitly (each copy takes more space), while `Multi<T>` encodes multiplicities in a separate bit vector of counts, so it can store large counts efficiently.
- [BitVecBuilder](src/bitvec/mod.rs) and [MultiBitVecBuilder](src/bitvec/mod.rs) are builder traits corresponding to the two traits above.
- [ByteSize](src/bytesize.rs) and [SymbolSequence](src/symbol_sequence.rs) are described above.

These traits enable writing code that is parametric over any particular bit vector type. For convenience, the builders have access to their target bit vector type as an [associated type](https://doc.rust-lang.org/rust-by-example/generics/assoc_items/types.html), and the bit vectors similarly have access to their builder type, which helped greatly when writing parametric test functions and enabled reusing test code across all concrete implementations of these traits.

Each builder type also contains an associated type describing the valid configuration options for its bit vector type, which turned out to be a nice way to enable customizability while maintaining a coherent interface.

To turn a MultiBitVec into a BitVec, see [BitVecOf](src/bitvec/mod.rs).

## WebAssembly bindings

This package provides experimental WebAssembly bindings to its bit vectors and the wavelet matrix, implemented in `js.rs`. The bindings use another package I wrote, [to_js](https://github.com/iopsystems/to_js), which implements basic Rust–JS bindings in around 750 lines of Rust.

These bindings are gated behind the `js` Cargo feature (off by default) and currently require a local path dependency for `to_js`, so the default build configuration in `Cargo.toml` omits them. Re-enabling involves restoring the `to_js` dependency line and adding `js = ["dep:to_js"]` to the feature table.

## Testing

- Exhaustive sweeps over small universes using [Exhaustigen](https://github.com/graydon/exhaustigen-rs), each cross-checked against `ArrayBitVec` as the slow reference.
- A deterministic boundary-universe-size sweep at the powers-of-2 edges (0, 1, 31, 32, 33, ..., 1024, 1025) crossed with seven density patterns.
- Randomized property tests via [arbtest](https://github.com/matklad/arbtest) — rank/select monotonicity and round-trip, Kraft equality of Huffman lengths, prefix-freeness of WM codes, BIC encode/decode round-trip, `quantile_batch` vs. per-k `quantile`, `majority(k)` vs. a naive count, and more.
- The [SymbolSequence](src/symbol_sequence.rs) shared property suite cross-validates `WaveletMatrix` and `HuffmanWaveletMatrix` on identical data — every overlapping query must agree.
- [expect-test](https://github.com/rust-analyzer/expect-test) inline snapshots for stable representational outputs (Huffman code pipeline, DenseBitVec block layout).

## Benchmarks

Criterion benchmarks in `benches/`: `rank1.rs` and `bitvec.rs` (per-bitvec rank/select sweeps, including dense/cache-cold select workloads), `wavelet_matrix.rs` (get/count/quantile/quantile_batch/count_batch), `huffman_vs_wm.rs` (HuffmanWM vs. plain WM on Zipf data), and `bic.rs` (encode/decode throughput across four distributions).

## Future work

- Add compressed bit vectors as described in [Fast, Small, Simple Rank/Select on Bitmaps](https://users.dcc.uchile.cl/~gnavarro/ps/sea12.1.pdf).
- Add quad vectors and the quad wavelet matrix; explore its use for two-dimensional range queries without the need for Morton masks.
  - Paper: [Faster wavelet trees with quad vectors](https://www.kurpicz.org/assets/publications/qwm_preprint.pdf)
  - Paper: [Faster Wavelet Tree Queries](https://arxiv.org/abs/2302.09239)
  - Existing [QWT implementation](https://github.com/rossanoventurini/qwt)
- Extend the iterator API beyond `DenseBitVec::ones` / `select1_range` to other bit vector types where intermediate state is genuinely shared across yields.
- Add quantile, counts, count_batch, and select_first_less_than on the Huffman wavelet matrix. Variable-length codes complicate level-by-level traversal but several of these are tractable.
