# Made of Bits

A little library of bit-based data structures.

Still under construction, and currently composed primarily of bit vectors designed for efficient rank and select operations over static data, with each implementation obeying the same abstract interface but specialized for a different data distribution: bitsets with and without repetition, dense, sparse, and run-length encoded.

The near- to medium-term future plans include documentation of the existing structures, the addition of basic benchmarking infrastructure to gather performance data, and the addition of a [wavelet matrix](https://www.sciencedirect.com/science/article/pii/S1570866713000610) with batch-oriented functionality.
