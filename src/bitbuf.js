import { assert, assertSafeInteger } from './assert.js';
import * as bits from './bits.js';
import { u32 } from './bits.js';
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

  /**
   * @param {number} threshold - a number in [0, 1] controlling when to zero-compress
   * if we "compressed" the blocks to less than `threshold` % of the original
   * number of blocks, return the zero-padded buffer; otherwise, return the original.
   * Set to 1.0 to always compress, or eg. 0.95 to compress if there is a 5% savings
   * or more in the number of blocks. Set this to 0.0 to only compress if the vector
   * is entirely full of zeros.
   */
  maybePadded(threshold = 1.0) {
    const zp = new PaddedBitBuf(this);
    const numCompressedBlocks = zp.right - zp.left;
    if (numCompressedBlocks / this.numBlocks <= threshold) {
      return zp;
    } else {
      return this;
    }
  }
}

/**
 * @param {number[] | Uint32Array} array
 * @param {number} value
 */
function countPadding(array, value) {
  // Compute the left and right indices of the blocks
  // we would like to keep, ie. which are non-padding.
  let left = 0;
  while (left < array.length && array[left] === value) {
    left++;
  }
  let right = array.length - 1;
  while (right > 0 && array[right] === value) {
    right--;
  }
  // in the returned object, left is inclusive and right is exclusive
  return { left, right: right + 1 };
}

/**
 * Immutable buffer with zero padding on both ends. Effectively RLE-encodes zeros
 * at the start and end of a BitBuf, allowing us to save space when the buffer begins
 * and/or ends with a significant number of zeros. The padding is block-aligned.
 */
export class PaddedBitBuf {
  /**
   * @param {BitBuf} buf
   */
  constructor(buf) {
    const { blocks, universeSize, numBlocks, numTrailingUnownedZeros } = buf;

    const zeroBlockPadding = 0;
    const zero = countPadding(blocks, zeroBlockPadding);
    const zeroLen = zero.right - zero.left; // number of non-padding blocks

    const oneBlockPadding = bits.oneMask(bits.BasicBlockSize);
    const one = countPadding(blocks, oneBlockPadding);
    const oneLen = one.right - one.left;

    // pick the padding that results in the shorter blocks array, or zero in case of a tie.
    const padding = zeroLen <= oneLen ? u32(0) : u32(1);
    const blockPadding = padding === 0 ? zeroBlockPadding : oneBlockPadding;
    let { left, right } = padding === 0 ? zero : one;

    if (left === 0 && right === blocks.length) {
      this.blocks = blocks;
    } else {
      this.blocks = blocks.slice(left, right);
    }
    this.left = left;
    this.right = right;
    this.padding = padding;
    this.blockPadding = blockPadding;

    // These two properties are transferred from the original BitBuf
    // without modification, since they form part of the public interface.
    // Their meaning refers to the original BitBuf.
    this.numTrailingUnownedZeros = numTrailingUnownedZeros;
    this.numBlocks = numBlocks;
    this.universeSize = universeSize;
  }

  /**
   * @param {number} index - block index
   */
  getBlock(index) {
    DEBUG && assertSafeInteger(index);
    DEBUG && assert(index >= 0 && index < this.numBlocks, "invalid block index");
    if (index < this.left || index >= this.right) return this.blockPadding;
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
    if (index < this.left || index >= this.right) return this.padding;
    index -= this.left;

    const block = this.blocks[index];
    const bit = block & (1 << bits.basicBlockBitOffset(bitIndex));
    return bit === 0 ? 0 : 1;
  }
}
