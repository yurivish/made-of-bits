import { DEBUG, assert, assertSafeInteger } from './assert.js';
import { BLOCK_BITS, blockBitOffset, blockIndex } from './bits.js';

// todo: class docstring

export class BitBuf {
  /**
   * @param {number} lengthInBits
   */
  constructor(lengthInBits) {
    const numBlocks = Math.ceil(lengthInBits / BLOCK_BITS);
    this.blocks = new Uint32Array(numBlocks);
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
    return bit !== 0;
  }

  /**
   * @param {number} bitIndex
   */
  set(bitIndex) {
    DEBUG && assertSafeInteger(bitIndex);
    DEBUG && assert(bitIndex >= 0 && bitIndex < this.lengthInBits);
    const index = blockIndex(bitIndex);
    const block = this.blocks[index];
    const bit = 1 << blockBitOffset(bitIndex);
    this.blocks[index] = block | bit;
  }
}

