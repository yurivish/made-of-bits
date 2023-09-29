import { assert, assertSafeInteger } from './assert.js';
import * as bits from './bits.js';
import './debug.js';

/**
 * Fixed-size bit buffer. Designed to be written once and read many times.
 * Conceptually, this is a special case of the fixed-width IntBuf.
 */
export class BitBuf {
  /**
   * Construct a new `BitBuf` containing all 0-bits.
   * @param {number} universeSize - the length of this bit buffer, in bits
 */
  constructor(universeSize) {
    assertSafeInteger(universeSize);
    assert(universeSize >= 0);
    const numBlocks = Math.ceil(universeSize / bits.BasicBlockSize);

    /** @readonly */
    this.blocks = new bits.BasicBlockArray(numBlocks);

    /** @readonly */
    this.universeSize = universeSize;

    const lastBlockOccupancy = universeSize % bits.BasicBlockSize;
    
    /** 
     * Number of trailing zeros in the final block that do not belong to this buffer
     * @readonly */
    this.numTrailingZeros = lastBlockOccupancy === 0 ? 0 : bits.BasicBlockSize - lastBlockOccupancy;
  } 

  /** 
   * @param {number} bitIndex
   */
  get(bitIndex) {
    DEBUG && assertSafeInteger(bitIndex);
    DEBUG && assert(bitIndex >= 0 && bitIndex < this.universeSize);
    const block = this.blocks[bits.basicBlockIndex(bitIndex)];
    const bit = block & (1 << bits.basicBlockBitOffset(bitIndex));
    return bit === 0 ? 0 : 1;
  }

  /**
   * Set the bit at index `bitIndex` to a 1-bit.
   * @param {number} bitIndex
   */
  setOne(bitIndex) {
    DEBUG && assertSafeInteger(bitIndex);
    DEBUG && assert(bitIndex >= 0 && bitIndex < this.universeSize);
    const blockIndex = bits.basicBlockIndex(bitIndex);
    const block = this.blocks[blockIndex];
    const bit = 1 << bits.basicBlockBitOffset(bitIndex);
    this.blocks[blockIndex] = block | bit;
  }

  /**
   * Set the bit at index `bitIndex` to a 0-bit.
   * @param {number} bitIndex
   */
  setZero(bitIndex) {
    DEBUG && assertSafeInteger(bitIndex);
    DEBUG && assert(bitIndex >= 0 && bitIndex < this.universeSize);
    const blockIndex = bits.basicBlockIndex(bitIndex);
    const block = this.blocks[blockIndex];
    const bit = 1 << bits.basicBlockBitOffset(bitIndex);
    this.blocks[blockIndex] = block & ~bit;
  }
}

