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
    // todo: improve these error messages
    assert(srPow2 >= bits.BLOCK_BITS_LOG2, 'rank sample rate 2^srPow2 must be a nonnegative multiple of the block size');
    assert(ssPow2 >= bits.BLOCK_BITS_LOG2, 'select sample rate 2^ssPow2 must be a nonnegative multiple of the block size');

    const ss = 1 << ssPow2; // Select sampling rate: sample every `ss` 1-bits
    const sr = 1 << srPow2; // Rank sampling rate: sample every `sr` bits

    this.data = data;
    this.srPow2 = srPow2;
    this.ssPow2 = ssPow2;
  }
}