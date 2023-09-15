import { BLOCK_BITS, blockBitOffset, blockIndex } from './bits.js';

// todo: class docstring

export class BitBuf {
  /**
   * @param {number} length - buffer length in bits
   */
  constructor(length) {
    const numBlocks = Math.ceil(length / BLOCK_BITS);
    this.blocks = new Uint32Array(numBlocks);
    this.length = length;
  }

  /**
   * @param {number} bitIndex
   */
  get(bitIndex) {
    const block = this.blocks[blockIndex(bitIndex)];
    const bit = block & (1 << blockBitOffset(bitIndex));
    return bit !== 0;
  }

  /**
   * @param {number} bitIndex
   */
  set(bitIndex) {
    const index = blockIndex(bitIndex);
    const block = this.blocks[index];
    const bit = 1 << blockBitOffset(bitIndex);
    this.blocks[index] = block | bit;
  }
}

