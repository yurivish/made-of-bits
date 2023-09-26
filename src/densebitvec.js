
// todo:
// - transfer more comments from the rust code
// - is it weird that all the "BLOCK_BITS" functions are in "bits"
//   and not bitbuf? Isn't it the bit buf that actually has the blocks?
//   It only makes sense if other types use the same blocks, and I guess
//   the IntBuf also uses them. But this should be clearly documented & explained!
// - note the precise meaning of s0 and s1. Does each select sample point at the block containing its bit?
// - note the precise meaning of r; is it the number of ones *preceding* this block?
// - call this RankSelect since it does not store the data itself (rather accepts a bitbuf)
//   but augments it with fast rank/select indexes?
// - i kinda wanna visualize when select samples are taken – right now I don't really understand it...
//   and this would be useful for a writeup of this technique.
// - decide whether to use the Pow2 suffix or the _LOG2 suffix or something else, but be consistent with both
// - consistently order Select0 and Select1 handling (probably 1 before 0 since rank1/select1 pull focus)
// - Investigate using u32 to also type case to a nominal U32 type.
// - for select1/select0, explore binary search over the suffix of rank samples (once we have benchmarking set up):
//   we want the rightmost rank index such that r1[rankIndex] <= n.
// - exciting question: what could it look like to do batch-select on a sorted set of ns?
//   - maybe transducers are involved
//   - figure out how to access each piece of data just once - eg. never re-access a rank block.
//   - i think nested for loop where each inner loop increments ns[i] while it is within the same type of block.
//   - we can check times against simple-sds: https://github.com/jltsiren/simple-sds
//     - it has bv-benchmark and wm-venchmark: https://github.com/jltsiren/simple-sds/blob/main/src/bin/bv-benchmark/main.rs
// - read this good post on array performance in v8: https://v8.dev/blog/elements-kinds
// - implement quad vectors: https://arxiv.org/abs/2302.09239
//   - 128 bit superblock: for each of (00, 01, 10, 11), store # of occurrences 
// - if DEBUG, append all algorithm steps to a log, cleared upon execution of the next algorithm.
//   - eg. an algorithm is select1 or rank1.
// - allow specifying select0SamplesPow2 separately from select1SamplesPow2
// - visualize the results of this algorithm to verify that this increases
//   the efficiency of the search. also, benchmark to verify that the additional
//   steps & memory accesses do not slow down the algorithm even with reduced search space.

import { assert, assertNotUndefined, assertSafeInteger, log } from "./assert.js";
import { BitBuf } from './bitbuf.js';
import * as bits from './bits.js';

/**
 * @implements {BitVecBuilder}
 */
export class DenseBitVecBuilder {
  /**
   * @param {number} universeSize
   */
  constructor(universeSize) {
    this.buf = new BitBuf(universeSize);
  }

  /**
   * @param {number} index
   */
  one(index, count = 1) {
    assert(count === 1);
    this.buf.setOne(index);
  }

  build({ rank1SamplesPow2 = 10, selectSamplesPow2 = 10 } = {}) {
    return new DenseBitVec(this.buf, rank1SamplesPow2, selectSamplesPow2);
  }
}

/** 
 * Dense bit vector with rank and select, based on the ideas described in the paper
 * 
 *   - Title: Fast, Small, Simple Rank/Select on Bitmaps
 *   - Authors: Gonzalo Navarro and Eliana Providel
 *   - Link: https://users.dcc.uchile.cl/~gnavarro/ps/sea12.1.pdf
 * 
 * We implement their structure for plain bitmaps. We use 32-bit blocks rather than
 * the 8-bit blocks as described in the paper, but otherwise the ideas are the same. 
 *
 * @implements BitVec 
 * */
export class DenseBitVec {
  /**
   * @param {BitBuf} data - bit buffer containing the underlying bit data
   * @param {number} rank1SamplesPow2 - power of 2 of the rank sample rate
   * @param {number} selectSamplesPow2 - power of 2 of the select sample rate for both select0 and select1
   */
  constructor(data, rank1SamplesPow2, selectSamplesPow2) {
    // todo: 
    // - kw args for sampling rates, with 2^10 being default
    // - Accept s0Pow2, s1Pow2 instead of ssPow2 in order to control the space usage; 
    //   the s0 index only matters for select0, while select1 helps speed up rank1 and rank0.
    // - document the meanings of the values of the r/s0/s1 arrays.
    // - update all references to 'raw' blocks – those mean the blocks in the bitbuf.
    // - assert there are less than 2^32 bits (since we do bitwise ops on bit indexes)
    assertSafeInteger(rank1SamplesPow2); 
    assertSafeInteger(selectSamplesPow2);
    assert(rank1SamplesPow2 >= bits.BlockSizePow2, 'sr must be a positive multiple of the block size');
    assert(selectSamplesPow2 >= bits.BlockSizePow2, 'ss must be a positive multiple of the block size');

    const select1SampleRate = 1 << selectSamplesPow2; // Select1 sampling rate: sample every `ss1` 1-bits
    const select0SampleRate = 1 << selectSamplesPow2; // Select0 sampling rate: sample every `ss0` 0-bits
    const rank1SampleRate = 1 << rank1SamplesPow2; // Rank sampling rate: sample every `sr` bits

    // Distinguish
    // - Which bit (position) the sample represents
    // - What it stores about that (or related) bit positions
    // - Bits vs blocks; we want our stuff block-aligned

    // Each rank sample identifies a particular basic block. 
    // 

    // Rank samples are sampled every `rank1SamplingRate` bits, where `rank1SamplingRate` is a positive multiple of
    // the bit width of a basic block. For example, if `rank1SamplingRate` is 64 and the basic
    // block width is 32, then the rank samples will tell us about the 0th, 2nd, 4th, 6th, ... basic block.
    //
    // A rank sample `rank1Samples[i]` tells us about the basic block `data.blocks[i << (srPow2 - bits.BLOCK_BITS_LOG2)]`.
    //
    // If `rank1Samples[i] has value `v`, this means that there are `v` 1-bits preceding that basic block.
    // Rank samples represent the number of 1-bits up to but not including a basic block.
    const rank1Samples = []; 

    // Each select1 sample identifies a particular basic block.
    //
    // Select samples are sampled every `select1SampleRate` 1-bits, where `rank1SamplingRate` is a positive multiple of
    // the bit width of a basic block. Unlike rank blocks, which start sampling from 0 (representing the 
    // `rank1SamplingRate*i + 0`-th bits), select blocks start sampling from 1, and thus represent the
    // `select1SamplingRate*i + 1`-th bits.
    // For example, if `select1SamplingRate` is 64, then the select1 samples will identify the basic blocks
    // that contain the 0+1 = 1st, 64+1 = 65th, 2*64+1 = 129th, 3*64+1 = 193rd, ... bits. Note that since
    // the sampling rate is a positive multiple of the basic block, two select block will never point 
    // to the same basic block.
    const select1Samples = []; 
    const select0Samples = []; 

    // Select1 samples represent the number of 1-bits up to but not including a basic block.
    // For example, if `select1SamplingRate`
    // is 64, then the select1 samples will tell us about the basic blocks containing the 1st
    // A select sample `select1Samples[i]` tells us about the basic block that contains the
    // `selectSamplingRate * i + 1`-th 1-bit.

    let cumulativeOnes = 0; // 1-bits preceding the current raw block
    let cumulativeBits = 0; // bits preceding the current raw block
    let zerosThreshold = 0; // take a select0 sample at the (zerosThreshold+1)th 1-bit
    let onesThreshold = 0; // take a select1 sample at the (onesThreshold+1)th 1-bit

    const basicBlocksPerRank1Sample = rank1SampleRate >>> bits.BlockSizePow2;
    const blocks = data.blocks;

    const maxBlockIndex = blocks.length - 1;
    for (let blockIndex = 0; blockIndex < blocks.length; blockIndex++) {
      const block = blocks[blockIndex];
      if (blockIndex % basicBlocksPerRank1Sample === 0) {
        rank1Samples.push(cumulativeOnes);
      }

      const blockOnes = bits.popcount(block);
      let blockZeros = bits.BlockSize - blockOnes;
      // Don't count trailing zeros in the final data block towards the zero count
      if (blockIndex === maxBlockIndex) blockZeros -= data.numTrailingZeros;
      const cumulativeZeros = cumulativeBits - cumulativeOnes;


      // Sample 1-bits for the select1 index
      if (cumulativeOnes + blockOnes > onesThreshold) {
        // Take a select1 sample, which consists of two parts:
        // 1. The cumulative number of bits preceding this basic block, ie. the left-shifted block index.
        //    This is `cumulativeBits`, defined above, and is stored in the high bits.
        // 2. A correction factor storing the number of 1-bits preceding the (ss1 * i + 1)-th 1-bit within this
        //    basic block, which we can use to determine the number of 1-bits preceding this basic block.
        //    Effectively, this is a way for us to store samples that are slightly offset from the strictly
        //    regular select sampling scheme, enabling us to keep the select samples aligned to basic blocks.
        //    This is `correction`, and is stored in the low bits.
        const correction = onesThreshold - cumulativeOnes;
        // Since cumulativeBits is a multiple of the basic block size,
        // these two values should never overlap in their bit ranges.
        DEBUG && assert((cumulativeBits & correction) === 0);
        // Add the select sample and bump the onesThreshold.
        select1Samples.push(cumulativeBits | correction);
        onesThreshold += select1SampleRate;
      }

      // Sample 0-bits for the select0 index.
      // This `if` block has the same structure as the one above which samples 1-bits.
      if (cumulativeZeros + blockZeros > zerosThreshold) {
        const correction = zerosThreshold - cumulativeZeros;
        DEBUG && assert((cumulativeBits & correction) === 0);
        select0Samples.push(cumulativeBits | correction);
        zerosThreshold += select0SampleRate;
      }

      cumulativeOnes += blockOnes;
      cumulativeBits += bits.BlockSize;
    }

    /** @readonly */
    this.data = data;

    /** @readonly */
    this.rank1SamplesPow2 = rank1SamplesPow2;

    /** @readonly */
    this.select0SamplesPow2 = selectSamplesPow2;

    /** @readonly */
    this.select1SamplesPow2 = selectSamplesPow2;

    /** @readonly */
    this.rank1Samples = new Uint32Array(rank1Samples);

    /** @readonly */
    this.select0Samples = new Uint32Array(select0Samples);

    /** @readonly */
    this.select1Samples = new Uint32Array(select1Samples);

    /** @readonly */
    this.basicBlocksPerRank1SamplePow2 = rank1SamplesPow2 - bits.BlockSizePow2;

    /** @readonly */
    this.numOnes = cumulativeOnes;

    /** @readonly */
    this.numZeros = data.universeSize - cumulativeOnes;

    /** @readonly */
    this.universeSize = data.universeSize;
  }

  /**
   * @param {number} index
   */
  rank1(index) {
    if (index < 0) {
      return 0;
    } else if (index >= this.universeSize) {
      return this.numOnes;
    }

    // Start with the prefix count from the rank block
    let rankIndex = index >>> this.rank1SamplesPow2;
    let count = this.rank1Samples[rankIndex];
    let rankBasicBlockIndex = rankIndex << this.basicBlocksPerRank1SamplePow2;
    let lastBasicBlockIndex = index >>> bits.BlockSizePow2;

    // Scan any intervening select blocks to skip past multiple basic blocks at a time
    let selectSampleRate = 1 << this.select1SamplesPow2;

    // Synthesize a fictitious initial select sample located squarely at the position
    // designated by the rank sample.
    let selectSample = { basicBlockIndex: rankBasicBlockIndex, precedingCount: count };
    let selectCount = selectSample.precedingCount + selectSampleRate;
    while (selectCount < this.numOnes && selectSample.basicBlockIndex < lastBasicBlockIndex) {
      selectSample = this.selectSample(selectCount, this.select1Samples, this.select1SamplesPow2);
      if (selectSample.basicBlockIndex >= lastBasicBlockIndex) break;
      count = selectSample.precedingCount;
      rankBasicBlockIndex = selectSample.basicBlockIndex;
      selectCount += selectSampleRate;
    }

    // Increment the count by the number of ones in every subsequent block
    for (let i = rankBasicBlockIndex; i < lastBasicBlockIndex; i++) {
      count += bits.popcount(this.data.blocks[i]);
    }

    // Count any 1-bits in the last block up to `index`
    let bitOffset = bits.blockBitOffset(index);
    let maskedBlock = this.data.blocks[lastBasicBlockIndex] & bits.oneMask(bitOffset);
    count += bits.popcount(maskedBlock);
    return count;
  }

  /**
   * @param {number} index
   */
  rank0(index) {
    if (index < 0) {
      return 0;
    } else if (index >= this.universeSize) {
      return this.numZeros;
    };
    return index - this.rank1(index);
  }

  /**
   * @param {number} n
   */
  select1(n) {
    const result = this.maybeSelect1(n);
    if (result === null) {
      throw new Error(`n ${n} is not a valid 1-bit index`);
    }
    return result;
  }

  /**
   * @param {number} n
   */
  maybeSelect1(n) {
    if (n < 0 || n >= this.numOnes) return null;

    // Grab the basic block and count information from the select sample
    let { basicBlockIndex, precedingCount: count } = this.selectSample(n, this.select1Samples, this.select1SamplesPow2);
    assert(count <= n);

    if (DEBUG) {
      const prevRankIndex = basicBlockIndex >>> this.basicBlocksPerRank1SamplePow2;
      assert(prevRankIndex < this.rank1Samples.length);
    }

    // Scan any intervening rank blocks to skip past multiple basic blocks at a time
    let rankIndex = (basicBlockIndex >>> this.basicBlocksPerRank1SamplePow2) + 1;
    while (rankIndex < this.rank1Samples.length) {
      let nextCount = this.rank1Samples[rankIndex];
      if (nextCount > n) break;
      count = nextCount;
      basicBlockIndex = rankIndex << this.basicBlocksPerRank1SamplePow2;
      rankIndex++;
    }
    
    // Scan basic blocks until we find the one that contains the n-th 1-bit
    let basicBlock = 0;
    assert(basicBlockIndex < this.data.blocks.length); // the index is in-bounds for the first iteration
    while (basicBlockIndex < this.data.blocks.length) {
      basicBlock = this.data.blocks[basicBlockIndex];
      const nextCount = count + bits.popcount(basicBlock);
      if (nextCount > n) break;
      count = nextCount;
      basicBlockIndex++;
    }; 

    // Compute and return its bit index
    const blockBitIndex = basicBlockIndex << bits.BlockSizePow2;
    const bitOffset = bits.select1(basicBlock, n - count);
    return blockBitIndex + bitOffset;
  }

  /**
   * TODO: implement this with a nested loop, where we stride efficiently over
   * blocks of various kinds.
   * 
   * @param {number[]} n - sorted array of input 1-bit indices
   * @param {number[]} out - sorted array of output bit indices
   */
  select1Batch(n, out) {
    // todo
  }

  /**
   * This implementation is adapted from on maybeSelect1 above.
   * @param {number} n
   */
  maybeSelect0(n) {
    if (n < 0 || n >= this.numZeros) return null;

    // Grab the basic block and count information from the select sample
    let { basicBlockIndex, precedingCount: count } = this.selectSample(n, this.select0Samples, this.select0SamplesPow2);
    assert(count <= n);

    if (DEBUG) {
      const prevRankIndex = basicBlockIndex >>> this.basicBlocksPerRank1SamplePow2;
      assert(prevRankIndex < this.rank1Samples.length);
    }

    // Scan rank blocks to skip past multiple basic blocks at a time
    let rankIndex = (basicBlockIndex >>> this.basicBlocksPerRank1SamplePow2) + 1;
    while (rankIndex < this.rank1Samples.length) {
      let nextCount = (rankIndex << this.rank1SamplesPow2) - this.rank1Samples[rankIndex];
      if (nextCount > n) break;
      count = nextCount;
      basicBlockIndex = rankIndex << this.basicBlocksPerRank1SamplePow2;
      rankIndex++;
    }
    
    // Scan basic blocks until we find the one that contains the n-th 1-bit
    let basicBlock = 0;
    assert(basicBlockIndex < this.data.blocks.length); // the index is in-bounds for the first iteration
    while (basicBlockIndex < this.data.blocks.length) {
      basicBlock = this.data.blocks[basicBlockIndex];
      const nextCount = count + bits.popcount(~basicBlock);
      if (nextCount > n) break;
      count = nextCount;
      basicBlockIndex++;
    }; 

    // Compute and return its bit index
    const blockBitIndex = basicBlockIndex << bits.BlockSizePow2;
    const bitOffset = bits.select1(~basicBlock, n - count);
    return blockBitIndex + bitOffset;
  }

  /**
   * @param {number} n
   */
  select0(n) {
    const result = this.maybeSelect0(n);
    if (result === null) {
      throw new Error('n is not a valid 0-bit index');
    }
    return result;
  };

  /**
   * @param {number} n - we are looking for the n-th bit of the particular kind (1-bit or 0-bit)
   * @param {number} sr - power of 2 of the sample rate
   * @param {Uint32Array} samples - array of samples
   */
  selectSample(n, samples, sr) {
    DEBUG && assert(0 <= n && n <= this.universeSize);
    const sampleIndex = n >>> sr;
    DEBUG && assert(sampleIndex < samples.length);
    const sample = samples[sampleIndex];

    // bitmask with the Raw::BLOCK_BITS_LOG2 bottom bits set.
    const mask = bits.BlockSize - 1;
    
    // The cumulative number of bits preceding the identified basic block, 
    // ie. the left-shifted block index of that block.
    const cumulativeBits = sample & ~mask; // high bits

    // NOTE: The references to 1-bits below are written from the perspective of select1.
    // If using this function for select zero, think of "1-bit" as "0-bit".

    // The number of 1-bits in the identified basic block preceding the (select1SampleRate*i+1)-th 1-bit
    const correction = sample & mask; // low bits

    // number of 1-bits preceding the identified basic block.
    // The first term tells us the number of 1-bits preceding this select sample,
    // since the k-th sample represents the (k*sr + 1)-th bit and this tells us the (k*sr)-th
    // The second term allows us to identify how may 1-bits precede the basic block containing
    // the bit identified by this select sample.
    const precedingCount = (sampleIndex << sr) - correction;

    return {
      basicBlockIndex: cumulativeBits >>> bits.BlockSizePow2,
      precedingCount
    };
  }


  /**
   * Get the value of the bit at the specified index (0 or 1).
   * 
   * @param {number} index
   */
  get(index) {
    assert(index >= 0 && index <= this.universeSize);
    const value = this.rank1(index + 1) - this.rank1(index);
    DEBUG && assert(value === 0 || value === 1, `got ${value}, expected 1 or 0`);
    return value;
  }
};
