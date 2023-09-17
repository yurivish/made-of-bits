// Dense bit vector with rank and select, based on the ideas described
// in the paper "Fast, Small, Simple Rank/Select on Bitmaps".
// We use an additional level of blocks provided by BitVec, but the ideas are the same.

// todo:
// - transfer more comments from the rust code
// - is it weird that all the "BLOCK_BITS" functions are in "bits"
//   and not bitbuf? Isn't it the bit buf that actually has the blocks?
//   It only makes sense if other types use the same blocks, and I guess
//   the IntVec also uses them. But this should be clearly documented & explained!
// - note the precise meaning of s0 and s1. Does each select sample point at the block containing its bit?
// - note the precise meaning of r; is it the number of ones *preceding* this block?
// - fix all snake_case to camelCase
// - call this RankSelect since it does not store the data itself (rather accepts a bitbuf)
//   but augments it with fast rank/select indexes?
// - i kinda wanna visualize when select samples are taken – right now I don't really understand it...
//   and this would be useful for a writeup of this technique.

// Select samples are of the (ss*i+1)-th bit. Eg. if ssPow2 = 2, we sample the 1, 1+4=5, 5+4=9, 9+4=13, 13+4=17-th bits.
// So the first select block points to the raw block containing the 1st bit.
// The second block points to the raw block containing the 5th bit.
// And it also stores correction info, since eg. maybe that raw block has 2 set bits in it before the 5th bit.
// 
// Yeah. I think I want a visualization of the state after this constructor has run.
// Which means bundling this and serving it to a notebook (maybe esbuild does cors)

// I wonder if types can help disentangle my confusion with regard to the varieties of indexes and block types that are floating around.
// The runtime nature of the type system might make it impossible to use a newtype-like pattern.

import { DEBUG, assert, assertSafeInteger, log } from "./assert.js";
import { BitBuf } from './bitbuf.js';
import * as bits from './bits.js';

export class DenseBitVec {
  /**
   * @param {BitBuf} data - bit buffer containing the underlying bit data
   * @param {number} srPow2 - power of 2 of the rank sample rate
   * @param {number} ssPow2 - power of 2 of the select sample rate
   */
  constructor(data, srPow2, ssPow2) {
    // todo: 
    // - Accept s0Pow2, s1Pow2 instead of ssPow2 in order to control the space usage; 
    //   the s0 index only matters for select0, while select1 helps speed up rank1 and rank0.
    // - document the meanings of the values of the r/s0/s1 arrays.
    // - update all references to 'raw' blocks – those mean the blocks in the bitbuf.
    // - assert there are less than 2^32 bits (since we do bitwise ops on bit indexes)
    assertSafeInteger(srPow2); 
    assertSafeInteger(ssPow2);
    assert(srPow2 >= bits.BLOCK_BITS_LOG2, 'sr must be a positive multiple of the block size');
    assert(ssPow2 >= bits.BLOCK_BITS_LOG2, 'ss must be a positive multiple of the block size');

    const ss1 = 1 << ssPow2; // Select1 sampling rate: sample every `ss1` 1-bits
    const ss0 = 1 << ssPow2; // Select0 sampling rate: sample every `ss0` 0-bits
    const sr = 1 << srPow2; // Rank sampling rate: sample every `sr` bits

    const r = []; // rank samples
    const s0 = []; // select0 samples
    const s1 = []; // select1 samples

    let cumulativeOnes = 0; // 1-bits preceding the current raw block
    let cumulativeBits = 0; // bits preceding the current raw block
    let zerosThreshold = 0; // take a select0 sample at the (zerosThreshold+1)th 1-bit
    let onesThreshold = 0; // take a select1 sample at the (onesThreshold+1)th 1-bit

    let blocksPerRankSample = sr >> bits.BLOCK_BITS_LOG2;
    // log('blocksPerRankSample', blocksPerRankSample);

    // Iterate one rank block at a time for convenient rank sampling
    const blocks = data.blocks;
    for (let i = 0; i < blocks.length; i += blocksPerRankSample) {
      log('-----');
      log('sample:', i);
      r.push(cumulativeOnes);
      // iterate `j` through 0..<blocksPerRankSample and treat it as an index offset:
      // in the loop below, `i + j` is the index of the current block
      for (let j = 0; j < blocksPerRankSample && i + j < blocks.length; j++) {
        const block = blocks[i + j];
        const blockOnes = bits.popcount(block);
        const blockZeros = bits.BLOCK_BITS - blockOnes;
        const cumulativeZeros = cumulativeBits - cumulativeOnes;        
        log({ cumulativeBits, cumulativeOnes, cumulativeZeros, blockOnes, blockZeros, zeroTrigger: cumulativeZeros + blockZeros, zerosThreshold });

        // Sample 1-bits for the select1 index
        if (cumulativeOnes + blockOnes > onesThreshold) {
          log('s1 sample');
          // Take a select1 sample, which consists of two parts:
          // 1. The cumulative bits preceding this raw block, ie. left-shifted block index
          const high = cumulativeBits;
          // 2. The number of 1-bits before the (ss1 * i + 1)-th 1-bit within this raw block,
          //    which we can use to determine the number of 1-bits preceding this raw block.
          const low = onesThreshold - cumulativeOnes;
          // High is a multiple of the raw block size so these
          // two values should never overlap in their bit ranges.
          DEBUG && assert((high & low) === 0);
          // Add the select sample and bump the onesThreshold.
          s1.push(high + low);
          onesThreshold += ss1;
        }

        // Sample 0-bits for the select0 index.
        // This has the same shape as the code above for select1.
        if (cumulativeZeros + blockZeros > zerosThreshold) {
          log('s0 sample');
          // Take a select0 sample, which consists of two parts:
          // 1. The cumulative bits preceding this raw block
          const high = cumulativeBits;
          // 2. The number of 0-bits before (ss0 * i + 1)-th 0-bit within this raw block
          const low = zerosThreshold - cumulativeZeros;
          // High is a multiple of the raw block size so these
          // two values should never overlap in their bit ranges.
          DEBUG && assert((high & low) === 0);
          // Add the select sample and bump the zerosThreshold.
          log('push', s0, high, low);
          s0.push(high + low);
          zerosThreshold += ss0;
        }

        cumulativeOnes += blockOnes;
        cumulativeBits += bits.BLOCK_BITS;
      }
    }

    /** @readonly */
    this.data = data;

    /** @readonly */
    this.srPow2 = srPow2;

    /** @readonly */
    this.ssPow2 = ssPow2;

    /** @readonly */
    this.r = new Uint32Array(r);

    /** @readonly */
    this.s0 = new Uint32Array(s0);

    /** @readonly */
    this.s1 = new Uint32Array(s1);

    /** @readonly */
    this.numOnes = cumulativeOnes;
  }

  // todo: function to decode a single sample?

  // todo: document & explain what this does
  /**
   * Returns the information contained in the closest select sample
   * preceding the n-th sampled bit [todo: reword... least upper bound?]
   * Eg. we are looking for n=50. We may have a select sample representing
   * the 30th s-bit, saying it is at position 12345. { bitIndex: 12345, numOnes: 30 }
   * @param {Uint32Array} s
   * @param {number} ssPow2
   * @param {number} n - the n-th (0|1)-bit
   */
  selectSample(s, ssPow2, n) {
    // Select samples are taken every j*2^ssPow2 1-bits and stores
    // a value related to the bit position of the 2^ssPow2-th bit.
    // For improved performance, rather than storing the position of
    // that bit directly, each select sample holds two separate values:
    // 1. The raw-block-aligned bit position of that bit, ie. the number
    //    of bits preceding the raw block containing the 2^ssPow2-th bit.
    // 2. The bit position of the (ss * i + 1)-th 1-bit within that raw block,
    //    which we can subtract from j*2^ss_pow2 to tell the number of 1-bits
    //    up to the raw-block-aligned bit position.
    const sampleIndex = n >> ssPow2;
    const sample = s[sampleIndex];
    return this.decodeSelectSample(sample, sampleIndex << ssPow2);
  }

  /**
   * @param {number} sample
   * @param {number} sampleBitIndex
   */
  decodeSelectSample(sample, sampleBitIndex) {
    // bitmask with the Raw::BLOCK_BITS_LOG2 bottom bits set.
    const mask = bits.BLOCK_BITS - 1;
    const bitIndex = sample & ~mask;
    const correction = sample & mask;
    // assert that bit pos is data-block-aligned
    DEBUG && assert(bits.blockBitOffset(bitIndex) === 0);

    // Number of 1-bits represented by this sample, up to the raw block boundary
    // todo: these are not necessarily ONES; s may represent 0-samples (or even "01"-samples in the future)
    // todo: clarify if this is the count of 1-bits < bitIndex, or <= bitIndex
    // I now think this is the number of 1-bits *preceding* bitIndex.
    // But man this stuff is somehow very fiddly... I will feel better when I have a basic version passing tests,
    // because then I can work on simplifying this code while preserving correct behavior.
    const count = sampleBitIndex - correction;
    // our bit index is block-aligned, so we can just return it as a block index. 
    // todo: explain this more fully
    // todo: remove the fields we do not use from here
    return { bitIndex, dataBlockIndex: bitIndex >> bits.BLOCK_BITS_LOG2, rBlockIndex: bitIndex >> this.srPow2, count };
  }

  /**
   * @param {number} n
   */
  select1(n) {
    // We're looking for the bit index of the n-th 1-bit.
    // Call it the target bit.

    // If there is no n-th 1-bit, then return null.
    if (n >= this.numOnes) return null;

    // The target bit is somewhere in the data buffer. 
    // Use select samples to compute a lower bound on its position (bit index).
    // There are `count` 1-bits up to but not including index `bitIndex`.
    let { bitIndex, dataBlockIndex, rBlockIndex, count } = this.selectSample(this.s1, this.ssPow2, n);
    DEBUG && rBlockIndex === dataBlockIndex && assert(this.r[rBlockIndex] === count);
    // blockIndex (todo: rename for clarity) is a bitbuf block index.
    // so shift by 
    // log({ srPow2: this.srPow2, BLOCK_BITS_LOG2: this.BLOCK_BITS_LOG2 },);
    // log('waaao', bitIndex, blockIndex, blockIndex >>> (this.srPow2 - bits.BLOCK_BITS_LOG2));
    // assert(this.r[blockIndex << (this.srPow2 - bits.BLOCK_BITS_LOG2)] === count);
    // // there are `count` 1-bits up to but not including `bitIndex`, which is a rank-block-aligned index.

    // There may be some rank samples in between the lower bound and
    // the true position; iterate over rank blocks until we find the
    // last one whose count is less than or equal to n.

    // Use rank blocks to hop over many raw blocks.
    // This could use exponential search over [blockIndex, blocks.length);
    // depending on the query and bit distribution this could be an improvement.
    // Currently the worst case is linear search over rank samples.
    // index of the next rank block
    const blocks = this.data.blocks;
    let lastKnownGood = rBlockIndex;
    let lastKnownCount = count;
    rBlockIndex++;
    while (rBlockIndex < blocks.length && (count = this.r[rBlockIndex]) < n) {
      lastKnownGood = rBlockIndex;
      lastKnownCount = count;
      rBlockIndex++;
    }
    log(rBlockIndex);
    for (let i = rBlockIndex + 1; rBlockIndex < blocks.length; i++) {
      break;
    }

    // // Find the rank block right before the one that exceeds n. Or the final block, if none exceeds it.
    // while (blockIndex < blocks.length && blocks[blockIndex] < n) {
    //   const nextCount = blocks[blockIndex];
    //   if (nextCount < n) {
    //     count = nextCount;
    //     blockIndex++;
    //   }
    // }

    // hop raw blocks
  }

  // next:
  // - fn to decode a select block
  // - select1
  // - select0
  // - rank1
  // - rank0
  // - get
  // - numOnes
  // - universeSize
  // - get
}