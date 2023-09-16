import { DEBUG, assert, assertInteger, assertSafeInteger } from "./assert.js";
import { BitBuf } from './bitbuf.js';
import * as bits from './bits.js';

// note: writes proceed contiguously; more of a push operation.
// todo: class docstring
// An IntVec represents a fixed length vector of fixed width integers.
// A newly constructed IntVec will have the specified length and all elements will be initialized to 0.
// Elements can be added by pushing them onto the vector, which will add that element from the front at the lowest available index.
// In typical use, the vector will be initialized and and then precisely `length` elements will be pushed.

export class IntVec {
  /**
   * @param {number} length - the length of the IntVec (in elements)
   * @param {number} bitWidth
   */
  constructor(length, bitWidth) {
    assertInteger(bitWidth);
    // The bit width cannot exceed 32 bits because JavaScript's bit operations will
    // truncate to 32 bits before performing the operation. The bit width also cannot
    // exceed 2 * bits.BLOCK_BITS, but since BLOCK_BITS is 32 that condition is redundant.
    assert(bitWidth <= 32);
    assertInteger(length);

    const lengthInBits = length * bitWidth;
    const numBlocks = Math.ceil(lengthInBits / bits.BLOCK_BITS);
    this.data = new Uint32Array(numBlocks);

    this.bitWidth = bitWidth;
    this.length = length;

    this.lengthInBits = lengthInBits;
    this.writeCursor = 0; // in bits (todo: should that be in the variable name?)
    this.lowBitMask = bits.oneMask(bitWidth);
  }

  /**
   * Push a value into the IntVec.
   * Will throw an error if there is no room to store the value. 
   * Note that as a special case, this means that any number of
   * zeros can be pushed to a IntVec with bitWidth zero.
   * @param {number} value
   */
  push(value) {
    DEBUG && assertSafeInteger(value);
    DEBUG && assert(value < 2 ** this.bitWidth, 'value does not fit the bit width');
    DEBUG && assert(value >= 0, 'value must be positive');

    // todo: how should we handle pushing more than the expected number
    // of elements to a bitWidth===0 vector?
    // If we have zero bit width, only allow writing zeros (and there's no need to write them!)
    if (this.bitWidth == 0) {
      assert(value == 0, 'value must be zero if the bit width is zero');
      return;
    }
    console.log(this.writeCursor, this.lengthInBits);
    assert(this.writeCursor < this.lengthInBits, 'cannot push into a full IntVec');

    const index = bits.blockIndex(this.writeCursor);
    const offset = bits.blockBitOffset(this.writeCursor);

    // Number of bits available in the current block
    const numAvailableBits = bits.BLOCK_BITS - offset;

    DEBUG && assert(index < this.length);
    this.data[index] |= value << offset;
    
    // If needed, write any remaining bits into the next block.
    if (numAvailableBits < this.bitWidth) {
      DEBUG && assert(index + 1 < this.length);
      this.data[index + 1] = value >>> numAvailableBits;
    }

    this.writeCursor += this.bitWidth;
  }

  /**
   * @param {number} index
   */
  get(index) {
    DEBUG && assert(0 <= index && index < this.length, 'index must be in bounds');

    // If the bit width is zero, our vector is entirely full of zeros.
    if (this.bitWidth === 0) {
      return 0;
    }

    const bitIndex = index * this.bitWidth;
    const blockIndex = bits.blockIndex(bitIndex);
    const offset = bits.blockBitOffset(bitIndex);

    // Number of bits available in the current block
    const numAvailableBits = bits.BLOCK_BITS - offset;

    DEBUG && assert(blockIndex < this.length);
    let value = (this.data[blockIndex] & (this.lowBitMask << offset)) >>> offset;

    // If needed, extract the remaining bits from the bottom of the next block
    if (numAvailableBits < this.bitWidth) {
      const numRemainingBits = this.bitWidth - numAvailableBits;
      DEBUG && assert(blockIndex + 1 < this.length);
      const highBits = this.data[blockIndex + 1] & bits.oneMask(numRemainingBits);
      value |= highBits << numAvailableBits;
    }

    return value;
  }
}
