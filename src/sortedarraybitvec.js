import { assert, assertNonNegative, assertNotUndefined, assertSafeInteger, log } from "./assert.js";
import { partitionPoint } from './bits';
import * as defaults from './defaults';
import { ascending } from './sort.js';

/**
 * @implements {BitVecBuilder}
 */
export class SortedArrayBitVecBuilder {

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
    return new SortedArrayBitVec(this.ones, this.universeSize);
  }
}

// select: select the k-th occurrence of a 0/1 bit.
// rank: return the number of bits below "universe index" i
// todo: visualize the "stacked" image:
//
// bitvec:
//
//  bits:  1   1  1
// index: 0123456789
//
// multibitvec:
//
//      :         1
//      :  1      1
//  bits:  1   1  1
// index: 0123456789
//  rank: 0022223336
//
// sorted ones:
// [1, 1, 5, 8, 8, 8]
// 
/**
 * @implements {BitVec} 
 */
export class SortedArrayBitVec {
  /**
   * @param {number[]} ones
   * @param {number} universeSize
   */
  constructor(ones, universeSize) {
    let numUniqueOnes = 0;
    let hasMultiplicity = false;
    let prev = -1;
    for (let i = 0; i < ones.length; i++) {
      const cur = ones[i];
      hasMultiplicity ||= prev === cur;
      numUniqueOnes += Number(prev !== cur);
      assertNonNegative(cur);
      assertSafeInteger(cur);
      assert(prev <= cur, 'expected monotonically nondecreasing sequence');
      prev = cur;
    }

    /** @readonly */
    this.ones = ones;

    /** @readonly */
    this.universeSize = universeSize;

    /** @readonly */
    this.numOnes = ones.length;

    /** @readonly */
    this.numZeros = this.universeSize - numUniqueOnes;

    /** @readonly */
    this.hasMultiplicity = hasMultiplicity;

    /** @readonly */
    this.numUniqueOnes = numUniqueOnes;
    
    /** @readonly */
    this.numUniqueZeros = this.numZeros;

    /** @readonly */
    this.size = this.numZeros + this.numOnes;
  }

  /**
   * @param {number} index
   */
  rank1(index) {
    // Count and return the number of ones less than the given index.
    return partitionPoint(this.numOnes, i => this.ones[i] < index);
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
    if (n < 0 || n >= this.numOnes) {
      return null;
    }
    return this.ones[n];
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
   * @param {number} index
   */
  trySelect0(index) {
    // for some reason declaring the const type-checks, while returning
    // this value directly does not. Even though the same approach works
    // for rank0 (see above).
    const ret = defaults.trySelect0(this, index);
    return ret; 
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
    return defaults.get(this, index); 
  }
}