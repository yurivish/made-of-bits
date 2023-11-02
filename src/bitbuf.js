import { assert, assertSafeInteger } from './assert.js';
import * as bits from './bits.js';
import './debug.js';

// todo: Create a ZeroPaddedButBuf that is immutable and created with with a .trim() method
//       that finds the first and last one bits and trims off the excess, storing offsets.

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

    /** @readonly */
    this.numBlocks = this.blocks.length;

    const lastBlockOccupancy = universeSize % bits.BasicBlockSize;
    
    /** 
     * Number of trailing zeros in the final block that do not belong to this buffer
     * @readonly */
    this.numTrailingUnownedZeros = lastBlockOccupancy === 0 ? 0 : bits.BasicBlockSize - lastBlockOccupancy;
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
   * @param {number} index
   */
  getBlock(index) {
    DEBUG && assertSafeInteger(index);
    DEBUG && assert(index >= 0 && index < this.numBlocks, "invalid block index");
    return this.blocks[index];
  }

  toZeroPadded() {
    return new ZeroPaddedBitBuf(this);
  }
}

/**
 * Immutable buffer with zero padding on both ends. Effectively RLE-encodes zeros
 * at the start and end of a BitBuf, allowing us to save space when the buffer begins
 * and/or ends with a significant number of zeros. The padding is block-aligned.
 */
export class ZeroPaddedBitBuf {
  /**
   * @param {BitBuf} buf
   */
  constructor(buf) {
    const { blocks, universeSize, numBlocks, numTrailingUnownedZeros } = buf;

    // Compute the left and right indices of the blocks
    // we would like to keep, ie. which are nonzero.
    let left = 0;
    while (left < blocks.length && blocks[left] === 0) {
      left++;
    }

    let right = blocks.length - 1;
    while (right > 0 && blocks[right] === 0) {
      right--;
    }

    this.universeSize = universeSize;
    this.numTrailingUnownedZeros = numTrailingUnownedZeros;
    this.numBlocks = numBlocks; // including (conceptual) zero blocks

    this.blocks = blocks.slice(left, right + 1);
    this.left = left;
    this.right = right + 1;
  }

  /**
   * @param {number} index - block index
   */
  getBlock(index) {
    DEBUG && assertSafeInteger(index);
    DEBUG && assert(index >= 0 && index < this.numBlocks, "invalid block index");
    if (index < this.left || index >= this.right) return bits.u32(0);
    else return this.blocks[index - this.left];
  }

  /** 
   * @param {number} bitIndex - bit index
   */
  get(bitIndex) {
    DEBUG && assertSafeInteger(bitIndex);
    DEBUG && assert(bitIndex >= 0 && bitIndex < this.universeSize);

    // handle the left- and right-padded block indices separately,
    // and adjust the index to the stored blocks if the bit is inside
    // the padded region rather than a part of the padding.
    let index = bits.basicBlockIndex(bitIndex);
    if (index < this.left || index >= this.right) return 0;
    index -= this.left;

    const block = this.blocks[index];
    const bit = block & (1 << bits.basicBlockBitOffset(bitIndex));
    return bit === 0 ? 0 : 1;
  }
}
