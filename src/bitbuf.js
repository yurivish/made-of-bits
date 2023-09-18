import { DEBUG, assert, assertSafeInteger } from './assert.js';
import { BLOCK_BITS, blockBitOffset, blockIndex } from './bits.js';

/**
 * Fixed-size bit buffer. Designed to be written once and read many times.
 * Conceptually, this is a special case of the fixed-width IntBuf.
 * 
 */
export class BitBuf {
  /**
   * Construct a new `BitBuf` containing all 0-bits.
   * @param {number} lengthInBits
 */
  constructor(lengthInBits) {
    assertSafeInteger(lengthInBits);
    assert(lengthInBits >= 0);
    const numBlocks = Math.ceil(lengthInBits / BLOCK_BITS);

    /** @readonly */
    this.blocks = new Uint32Array(numBlocks);

    /** @readonly */
    this.lengthInBits = lengthInBits;
  } 

  /** 
   * @param {number} bitIndex
   */
  get(bitIndex) {
    DEBUG && assertSafeInteger(bitIndex);
    DEBUG && assert(bitIndex >= 0 && bitIndex < this.lengthInBits);
    const block = this.blocks[blockIndex(bitIndex)];
    const bit = block & (1 << blockBitOffset(bitIndex));
    return bit === 0 ? 0 : 1;
  }

  /**
   * Set the bit at index `bitIndex` to a 1-bit.
   * @param {number} bitIndex
   */
  setOne(bitIndex) {
    DEBUG && assertSafeInteger(bitIndex);
    DEBUG && assert(bitIndex >= 0 && bitIndex < this.lengthInBits);
    const index = blockIndex(bitIndex);
    const block = this.blocks[index];
    const bit = 1 << blockBitOffset(bitIndex);
    this.blocks[index] = block | bit;
  }
}

