import { assert, assertNonNegative, assertSafeInteger } from "./assert.js";
import { BitBuf } from './bitbuf.js';
import * as bits from './bits.js';
import * as defaults from './defaults';
import { DenseBitVec, DenseBitVecBuilder } from './densebitvec.js';
import { IntBuf } from './intbuf.js';
import { ascending } from './sort.js';

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

    // The paper "On Elias-Fano for Rank Queries in FM-Indexes" recommends a formula to compute
    // the number of low bits that is mostly equivalent to the version used below, except that
    // sometimes theirs suggests slightly worse choices, e.g. when numOnes === 25 and universeSize === 51.
    // https://observablehq.com/@yurivish/ef-split-points
    // This approach chooses the split point by noting that the trade-off effectively is between having numOnes
    // low bits, or the next power of two of the universe size separators in the high bits. Hopefully this will
    // be explained clearly in the accompanying design & background documentation.
    const numOnes = ones.length;
    const lowBitWidth = numOnes === 0 ? 0 : Math.floor(Math.log2(Math.max(1, universeSize / numOnes))); 

    // unary coding; 1 denotes values and 0 denotes separators, since that way
    // encoding becomes more efficient.
    // By default, values are never more than 50% of the bits due to the way the split point is chosen.
    // Note that this expression automatically adapts to non-power-of-two universe sizes.
    const highLength = numOnes + (universeSize >>> lowBitWidth);
    const high = new BitBuf(highLength);
    const low = new IntBuf(numOnes, lowBitWidth);
    const lowMask = bits.oneMask(lowBitWidth);

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
    // todo: explore passing a highBuilder here, so that you can eg. specify the options
    // to the bitvec type. We would have to change the above loop to use the builder, and
    // then say this.high = builder.build(buildOptions) with the options we were passed.
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
    return defaults.rank0(this, index);
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
    return defaults.select1(this, n);;
  }

  /**
   * @param {number} n
   */
  select0(n) {
    return defaults.select0(this, n);;
  };

  /**
   * @param {number} index
   */
  get(index) {
    return defaults.get(this, index);
  }
};