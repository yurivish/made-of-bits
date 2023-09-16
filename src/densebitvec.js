// Dense bit vector with rank and select, based on the ideas described
// in the paper "Fast, Small, Simple Rank/Select on Bitmaps".
// We use an additional level of blocks provided by BitVec, but the ideas are the same.

// todo:
// - transfer more comments from the rust code

import { DEBUG, assert, assertSafeInteger } from "./assert.js";
import { BitBuf } from './bitbuf.js';
import * as bits from './bits.js';

class DenseBitVec {
  /**
   * @param {BitBuf} data - bit buffer containing the underlying bit data
   * @param {number} srPow2 - power of 2 of the rank sample rate
   * @param {number} ssPow2 - power of 2 of the select sample rate
   */
  constructor(data, srPow2, ssPow2) {
    // todo: 
    // - Accept s0Pow2, s1Pow2 instead of ssPow2 in order to control the space usage; 
    //   the s0 index only matters for select0, while select1 helps speed up rank1 and rank0.
    assertSafeInteger(srPow2); 
    assertSafeInteger(ssPow2);
    assert(srPow2 >= bits.BLOCK_BITS_LOG2, 'sr must be a positive multiple of the block size');
    assert(ssPow2 >= bits.BLOCK_BITS_LOG2, 'ss must be a positive multiple of the block size');

    const ss = 1 << ssPow2; // Select sampling rate: sample every `ss` 1-bits
    const sr = 1 << srPow2; // Rank sampling rate: sample every `sr` bits

    const r = []; // rank samples
    const s0 = []; // select0 samples
    const s1 = []; // select1 samples

    let cumulativeOnes = 0; // 1-bits preceding the current raw block
    let cumulativeBits = 0; // bits preceding the current raw block
    let zerosThreshold = 0; // take a select0 sample at the (zerosThreshold+1)th 1-bit
    let onesThreshold = 0; // take a select1 sample at the (onesThreshold+1)th 1-bit

    // Iterate one rank block at a time for convenient rank sampling
    const blocks = data.blocks;
    for (let i = 0; i < blocks.length; i += sr) {
      r.push(cumulativeOnes);
      // iterate `j` through 0..<sr and treat it as an index offset:
      // in the loop below, `i + j` is the index of the current block
      for (let j = 0; j < sr; i + j < blocks.length) {
        const block = blocks[i + j];

        // Sample 1-bits for the select1 index
        const blockOnes = bits.popcount(block);
        if (cumulativeOnes + blockOnes > onesThreshold) {
          // Take a select1 sample, which consists of two parts:
          // 1. The cumulative bits preceding this raw block
          const high = cumulativeBits;
          // 2. The number of 1-bits before the (ss * i + 1)-th 1-bit within this raw block
          const low = onesThreshold - cumulativeOnes;
          // High is a multiple of the raw block size so these
          // two values should never overlap in their bit ranges.
          DEBUG && assert((high & low) === 0);
          // Add the select sample and bump the onesThreshold.
          s1.push(high + low);
          onesThreshold += ss;
        }

        // Sample 0-bits for the select0 index.
        // This has the same shape as the code above for select1.
        const blockZeros = bits.BLOCK_BITS;
        const cumulativeZeros = cumulativeBits - cumulativeOnes;
        if (cumulativeZeros + blockZeros > zerosThreshold) {
          // Take a select0 sample, which consists of two parts:
          // 1. The cumulative bits preceding this raw block
          const high = cumulativeBits;
          // 2. The number of 0-bits before (ss * i + 1)-th 0-bit within this raw block
          const low = zerosThreshold - cumulativeZeros;
          // High is a multiple of the raw block size so these
          // two values should never overlap in their bit ranges.
          DEBUG && assert((high & low) === 0);
          // Add the select sample and bump the zerosThreshold.
          s0.push(high + low);
          zerosThreshold += ss;
        }
      }
    }

    /** @readonly */
    this.data = data;

    /** @readonly */
    this.srPow2 = srPow2;

    /** @readonly */
    this.ssPow2 = ssPow2;

    /** @readonly */
    this.r = r;

    /** @readonly */
    this.s0 = s0;

    /** @readonly */
    this.s1 = s1;

    /** @readonly */
    this.numOnes = cumulativeOnes;
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