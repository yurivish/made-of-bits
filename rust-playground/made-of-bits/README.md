# Made of Bits

This library is the product of my explorations into [succinct data structures](https://en.wikipedia.org/wiki/Succinct_data_structure) for data visualization.
It implements several bit vector types with [rank and select operations](https://en.wikipedia.org/wiki/Succinct_data_structure#Succinct_indexable_dictionaries), each specialized to a particular data pattern:

- The [dense](https://github.com/yurivish/made-of-bits/blob/main/rust-playground/made-of-bits/src/bitvec/dense.rs) bit vector is based on the approach proposed in [Fast, Small, Simple Rank/Select on Bitmaps](https://www.dcc.uchile.cl/~gnavarro/ps/sea12.1.pdf).
- The [sparse](https://github.com/yurivish/made-of-bits/blob/main/rust-playground/made-of-bits/src/bitvec/sparse.rs) bit vector is implemented using [Elias-Fano encoding](https://www.antoniomallia.it/sorted-integers-compression-with-elias-fano-encoding.html).
- The [run-length-encoded](https://github.com/yurivish/made-of-bits/blob/main/rust-playground/made-of-bits/src/bitvec/rle.rs) bit vector uses an idea I developed together with [Gonzalo Navarro](https://users.dcc.uchile.cl/~gnavarro/), which is a tweak to a RLE bit vector described in his [book](https://www.amazon.com/Compact-Data-Structures-Practical-Approach/dp/1107152380) to improve the efficiency of rank queries. We wrote a [technical note](https://yuri.is/pdfing/weighted_range_quantile_queries.pdf) about it.
- The [array](https://github.com/yurivish/made-of-bits/blob/main/rust-playground/made-of-bits/src/bitvec/array.rs) bit vector is backed by a plain array, and serves as a useful baseline for correctness testing.


In addition, there are also a few wrapper types that grant new powers to an existing bit vector:
- [Multi\<T\>](https://github.com/yurivish/made-of-bits/blob/main/rust-playground/made-of-bits/src/bitvec/multi.rs) wraps a plain bit vector and turns it into a "multi bit vector" which stores counts and allows multiple repetitions of the same bit to be stored (if a bit vector is a set, a multi bit vector is a multiset).
- [ZeroPadded\<T\>](https://github.com/yurivish/made-of-bits/blob/main/rust-playground/made-of-bits/src/bitvec/zeropadded.rs) embeds a bit vector into a larger universe by padding it on the left and right with 0-bits. These extra bits take no space as they are represented implicitly.

## Wavelet matrix
The crown jewel of this crate is the [wavelet matrix](https://github.com/yurivish/made-of-bits/blob/main/rust-playground/made-of-bits/src/waveletmatrix.rs) data structure, which generalizes rank/select bit vectors to larger integer alphabets than (_0_, _1_). (The [wavelet matrix](https://users.dcc.uchile.cl/~gnavarro/ps/spire12.4.pdf) is a variant of the [wavelet tree](https://www.sciencedirect.com/science/article/pii/S1570866713000610) that improves space efficiency and runtime performance, particularly for large alphabets.)

This crate provides wavelet matrix rank and select, as well as a few other useful operations like range count and [range quantile](https://arxiv.org/abs/0903.4726).

Storing [Morton codes](https://en.wikipedia.org/wiki/Z-order_curve) in the wavelet matrix enables multi-dimensional range queries, which is great fun for zoomable density scatterplots with adaptive level-of-detail. This data representation is sometimes called an "[interleaved wavelet tree](https://diegocaro.cl/thesis/thesis.pdf)" and can be used for other spatial operations, though I've only implemented range count queries here.

## Traits
This crate defines several traits:
- [BitVec](https://github.com/yurivish/made-of-bits/blob/03b66e2ce37c9a1252670991726048156303a28f/rust-playground/made-of-bits/src/bitvec/mod.rs#L14) is implemented by plain bit vectors (which are functionally integer sets)
- [MultiBitVec](https://github.com/yurivish/made-of-bits/blob/03b66e2ce37c9a1252670991726048156303a28f/rust-playground/made-of-bits/src/bitvec/mod.rs#L99C11-L99C21) is implemented by bit vector types that support storing the same integer multiple times.  `ArrayBitVec` and `SparseBitVec` store repetitions explicitly (each copy takes more space), while `Multi<T>` encodes multiplicities in a bit vector, so it can store large counts efficiently.
- [BitVecBuilder](https://github.com/yurivish/made-of-bits/blob/03b66e2ce37c9a1252670991726048156303a28f/rust-playground/made-of-bits/src/bitvec/mod.rs#L137) and [MultiBitVecBuilder](https://github.com/yurivish/made-of-bits/blob/03b66e2ce37c9a1252670991726048156303a28f/rust-playground/made-of-bits/src/bitvec/mod.rs#L168) are builder traits corresponding to the two traits above.

These traits enable writing code that is parametric over any particular bit vector type. For convenience, the builders have access to their target bit vector type as an [associated type](https://doc.rust-lang.org/rust-by-example/generics/assoc_items/types.html), and the bit vectors similarly have access to their builder type, which helped greatly when writing parametric test functions and enabled reusing test code across all concrete implementations of these traits.

Each builder type also contains an associated type describing the valid configuration options for its bit vector type, which turned out to be a nice way to enable customizability while maintaining a coherent interface.

## WebAssembly bindings
This package provides experimental work-in-progress WebAssembly bindings to all of its bit vectors as well as the wavelet matrix, implemented in `js.rs`. The bindings use another package I wrote, [to_js](https://github.com/iopsystems/to_js), which implements basic Rust–JS bindings in around 750 lines of Rust. I didn't have a concrete use for the WebAssembly bindings when I was implementing this package so they're in a bit a proof of concept phase at the moment (but they do work!)

## Future work

- Add support for Huffman-compressed wavelet matrix construction and queries. (Tht top-level JS library in this repository implements this)
- Add support for compressed bit vectors as described in [Fast, Small, Simple Rank/Select on Bitmaps](https://users.dcc.uchile.cl/~gnavarro/ps/sea12.1.pdf)
  - See also: https://observablehq.com/d/5370347688e58b4d
- Add support for quad vectors and the quad wavelet matrix. Explore its use for two-dimensional range queries without the need for Morton masks.
  - Paper: [Faster wavelet trees with quad vectors](https://www.kurpicz.org/assets/publications/qwm_preprint.pdf) 
  - Paper: [Faster Wavelet Tree Queries](https://arxiv.org/abs/2302.09239)
  - Code for an existing [QWT implementation](https://github.com/rossanoventurini/qwt)
- Testing
  - Add more tests for rank1_batch, which is currently only spot-tested
  - Add tests for the individual bit vectors that capture the particular patterns each type is specialized for, and also test their configuration options.
    - various numbers of runs and run-lengths for rle, verifying the space savings
    - large universes and varying split points for sparse
    - varied densities and sampling options for dense
