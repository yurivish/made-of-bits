import { assert, assertNonNegative, assertSafeInteger } from "./assert.js";
import { BitBuf } from './bitbuf.js';
import * as bits from './bits.js';
import * as defaults from './defaults';
import { DenseBitVec, DenseBitVecBuilder } from './densebitvec.js';
import { IntBuf } from './intbuf.js';
import { ascending } from './sort.js';

// todo: decide how to choose the optimal split point (& document it)
/**
 * @implements {BitVecBuilder}
 */
export class SparseBitVecBuilder {

  /**
   * @param {number} universeSize
   */
  constructor(universeSize) {
    this.universeSize = universeSize;
    /** @type {number[]} */
    this.ones = [];
  }

  /**
   * @param {number} index
   */
  one(index, count = 1) {
    assert(index < this.universeSize, () => `index (${index}) cannot exceed universeSize (${this.universeSize})`);
    for (let i = 0; i < count; i++) {
      this.ones.push(index);
    }
  }
  
  build(options = {}) {
    this.ones.sort(ascending);
    return new SparseBitVec(this.ones, this.universeSize);
  }
}

/**
 * Sparse bitvector using Elias-Fano encoding. Supports multiplicity.
 * @implements {BitVec}
 */
export class SparseBitVec {
  /**
   * @param {number[] | Uint32Array | Float64Array} ones
   * @param {number} universeSize
   */
  constructor(ones, universeSize) {
    // disallow humungous universes because JS only supports efficient bit ops for 32-bit integers
    assert(universeSize < 2 ** 32, () => `universeSize (${universeSize}) cannot exceed 2^32 - 1`);
    // todo: understand the comments in the paper "On Elias-Fano for Rank Queries in FM-Indexes"
    // but for now do the more obvious thing. todo: explain.
    // this is nice because we don't need the number of high bits explicitly so can avoid computing them
    const numOnes = ones.length;
    const bitsPerOne = numOnes === 0 ? 0 : Math.floor(universeSize / numOnes);
    const lowBitWidth = Math.floor(Math.log2(Math.max(bitsPerOne, 1)));
    const lowMask = bits.oneMask(lowBitWidth);

    // unary coding; 1 denotes values and 0 denotes separators
    const highLength = numOnes + (universeSize >>> lowBitWidth);
    const high = new BitBuf(highLength);
    const low = new IntBuf(numOnes, lowBitWidth);

    let numUniqueOnes = 0;
    let hasMultiplicity = false;
    let prev = -1;
    for (let i = 0; i < ones.length; i++) {
      const cur = ones[i];
      hasMultiplicity ||= prev === cur;
      numUniqueOnes += Number(prev !== cur);
      assertNonNegative(cur);
      assertSafeInteger(cur);
      assert(cur < universeSize, () => `expected 1 - bit(${cur}) to not exceed the universeSize(${universeSize})`);
      assert(prev <= cur, 'expected monotonically nondecreasing sequence');
      prev = cur;

      // Encode element
      const quotient = cur >>> lowBitWidth;
      high.setOne(i + quotient);
      const remainder = cur & lowMask;
      low.push(remainder);
    }

    // todo: allow tuning of the block parameters
    /** @readonly */
    this.high = new DenseBitVec(high, 10, 10);

    /** @readonly */
    this.low = low;

    /** @readonly */
    this.numOnes = numOnes;

    /** @readonly */
    this.lowBitWidth = lowBitWidth;

    /** @readonly */
    this.lowMask = lowMask;

    /** @readonly */
    this.universeSize = universeSize;

    /** @readonly */
    this.numZeros = this.universeSize - numUniqueOnes;

    /** @readonly */
    this.hasMultiplicity = hasMultiplicity;

    /** @readonly */
    this.numUniqueOnes = numUniqueOnes;
    
    /** @readonly */
    this.numUniqueZeros = this.numZeros;

  }

  /**
   * @param {number} value
   */
  quotient(value) { 
    return value >>> this.lowBitWidth;
  }

  /**
   * @param {number} value
   */
  remainder(value) { 
    return value & this.lowMask;
  }

  /**
   * @param {number} index
   */
  rank1(index) {
    if (index < 0) {
      return 0;
    } else if (index >= this.universeSize) {
      return this.numOnes;
    }

    // Values are grouped by their upper bits, with the lower bits stored separately.
    // Compute the upper and lower bounds of the search range within the low bits,
    // which will tell us how many values within the upper bit group are below `index`.
    const quotient = this.quotient(index);
    let lowerBound = 0;
    let upperBound = 0;
    if (quotient === 0) {
      // We're searching within the first group, so the lower bound is zero.
      // Look for the divider that separates the first group from the subsequent groups.
      // If there isn't one, then we need to search the entire vector since all values
      // are in the first group.
      upperBound = this.high.trySelect0(0) ?? this.numOnes;
    } else {
      // We're searching within a higher group, so compute both the lower and the
      // upper bound from the high bit vector.
      {
        // We're searching for the i-th separator.
        // When we find it, we subtract the number of separators preceding it
        // in order to get the index of the element in the low bits.
        const i = quotient - 1;
        const n = this.high.trySelect0(i);
        lowerBound = n === null ? 0 : n - i;
      }

      {
        // Same thing, but we're searching for the next separator after that.
        const i = quotient;
        const n = this.high.trySelect0(i);
        upperBound = n === null ? this.numOnes : n - i;
      }
    }

    // Count the number of elements in this bucket that are strictly below i 
    // using just the low bits.
    const remainder = this.remainder(index);
    const bucketCount = bits.partitionPoint(upperBound - lowerBound, n => {
      const index = lowerBound + n;
      const value = this.low.get(index);
      return value < remainder;
    });

    return lowerBound + bucketCount;
  }

  /**
   * @param {number} index
   */
  rank0(index) {
    const result = defaults.rank0(this, index);
    return result;
  }

  /**
   * @param {number} n
   */
  trySelect1(n) {
    // How many zeros are there before the nth one bit?
    const pos = this.high.trySelect1(n);
    if (pos === null) {
      return null;
    }
    const quotient = this.high.rank0(pos);
    const remainder = this.low.get(n);
    return (bits.u32(quotient << this.lowBitWidth) + remainder);
  }

  /**
   * @param {number} n
   */
  trySelect0(n) {
    const result = defaults.trySelect0(this, n);
    return result;
  }

  /**
   * @param {number} n
   */
  select1(n) {
    const result = this.trySelect1(n);
    if (result === null) throw new Error(`n (${n}) is not a valid 1-bit index`);
    return result;
  }

  /**
   * @param {number} n
   */
  select0(n) {
    const result = this.trySelect0(n);
    if (result === null) throw new Error(`n (${n}) is not a valid 0-bit index`);
    return result;
  };

  /**
   * @param {number} index
   */
  get(index) {
    const result = defaults.get(this, index);
    return result;
  }

};;