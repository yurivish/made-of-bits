import { DEBUG, assert } from "./assert.js";
import { BitBuf } from './bitbuf.js';
import * as bits from './bits.js';

class DenseBitVec {
  /**
   * @param {BitBuf} data - bit buffer containing the underlying bit data
   * @param {number} srPow2 - power of 2 of the rank sample rate
   * @param {number} ssPow2 - power of 2 of the select sample rate
   */
  constructor(data, srPow2, ssPow2) {
    assert(srPow2 >= bits.BLOCK_BITS_LOG2, 'sr must be a positive multiple of the block size');
    assert(ssPow2 >= bits.BLOCK_BITS_LOG2, 'ss must be a positive multiple of the block size');

    const ss = 1 << ssPow2; // Select sampling rate: sample every `ss` 1-bits
    const sr = 1 << srPow2; // Rank sampling rate: sample every `sr` bits

    // Raw::BITS is bits.BLOCK_BITS
    // Raw::BIT_WIDTH is bits.BLOCK_BITS_LOG2

    const r = []; // rank samples
    const s0 = []; // select0 samples
    const s1 = []; // select1 samples

    let cumulativeOnes = 0; // 1-bits preceding the current raw block
    let cumulativeBits = 0; // bits preceding the current raw block
    let zerosThreshold = 0; // take a select0 sample at the (zerosThreshold+1)th 1-bit
    let onesThreshold = 0; // take a select1 sample at the (onesThreshold+1)th 1-bit

    // Blocks per rank sample
    let blockSamplingRate = sr >> bits.BLOCK_BITS_LOG2;
    // ^ was raw_block_sr (todo: remove this comment)


    this.data = data;
    this.srPow2 = srPow2;
    this.ssPow2 = ssPow2;
  }
}