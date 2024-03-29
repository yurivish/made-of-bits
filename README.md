# Made of Bits

A little library of bit-based data structures.

Still under construction, and currently composed primarily of bit vectors designed for efficient rank and select operations over static data, with each implementation obeying the same abstract interface but specialized for a different data distribution: bitsets with and without repetition, dense, sparse, and run-length encoded. It also contains an implementation of a wavelet matrix.

I've also been prototyping Huffman coding for the wavelet matrix, enabling compression up to the first-order entropy of the sequence. Making use of it requires adjusting the wavelet matrix construction algorithms to produce bit vectors of different lengths, and adjusting all query algorithms to stop descending down to the next level whenever a query winds up pointing at out of range values on the next level down. For construction, it's possible that the most straightforward approach is to create a new Huffman construction function using the bucket sort algorithm (it's less efficient, but is simpler.). The basic idea would be to take an additional vector of huffman code lengths, to right-pad the symbols with 1-bits (so they sort to the end), then gradually shorten the bit vector lengths as you go down the tree. You can see the code [here](https://github.com/yurivish/made-of-bits/blob/main/src/huffman.js).

I've also been experimenting with the programming language [Zig](https://ziglang.org). The `zig-playground` directory contains a draft implementation of a dense bit vector. The code produces a WASM file but needs accompanying JavaScript code to be usable. I have been prototyping that [here](https://observablehq.com/d/3cfad59903db0945). The Zig code currently does not do proper error handling and was very much a first pass at getting something working, so there are probably bugs and design infelicities.

The medium- to long-term future plans include making the above improvements to allow for Huffman shaped construction and querying, potentially porting performance-sensitive data structures to a lower-level language, documenting the existing structures, and adding basic benchmarking infrastructure to gather performance data. 

It would also be fun to add an implementation of quad vectors, which enable rank & select among the symbols 0, 1, 2, and 3. [This paper](https://arxiv.org/abs/2302.09239) outlines one possibility, but we could explore an extended version of the existing dense bit vector approach as an alternative, though there may be undesirable trade-offs there.
