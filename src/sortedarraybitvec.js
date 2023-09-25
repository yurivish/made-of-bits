import { assert, assertNotUndefined, assertSafeInteger, log } from "./assert.js";
import { partitionPoint } from './bits';
import * as sort from './sort.js';

// todo: test the multi bit vec nature of this type

/**
 * @implements {BitVecBuilder}
 */
export class SortedArrayBitVecBuilder {

  /**
   * @param {number} universeSize
   */
  constructor(universeSize) {
    this.universeSize = universeSize;
    /**
     * @type {number[]}
     */
    this.ones = [];
  }

  /**
   * @param {number} index
   */
  one(index, count = 1) {
    for (let i = 0; i < count; i++) {
      this.ones.push(index);
    }
  }
  
  build(options = {}) {
    this.ones.sort(sort.ascending);
    return new SortedArrayBitVec(this.ones, this.universeSize);
  }
}

// todo: implement the proper bitvec interface (Eg. universe size = #ones + #zeros)
// todo: decide how to handle multiplicity
// todo: figure out the appropriate "multi-bit-vec" interface. what does rank/select mean?
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
    // assert monotonically nondecreasing
    let hasMultiplicity = false;
    for (let i = 1; i < ones.length; i++) {
      const prev = ones[i - 1];
      const cur = ones[i];
      hasMultiplicity ||= prev === cur;
      assert(prev <= cur);
    }

    this.ones = ones;
    this.universeSize = universeSize;
    this.numOnes = ones.length;
    this.numZeros = this.universeSize - this.numOnes;
    this.hasMultiplicity = hasMultiplicity;
  }

  /**
   * @param {number} index
   */
  rank1(index) {
    return partitionPoint(this.universeSize, i => this.ones[i] < index);
  }

  // todo: needs to check for multiplicity:
  // debug_assert!(!self.has_multiplicity);
  /**
   * @param {number} index
   */
  rank0(index) {
    assert(!this.hasMultiplicity, 'cannot take rank0 in the presence of multiplicity (repeated elements)');
    if (index <= 0) {
      return 0;
    } else if (index >= this.universeSize) {
      return this.numZeros;
    };
    return index - this.rank1(index);
  }

  /**
   * @param {number} n
   */
  maybeSelect1(n) {
    if (n < 0 || n >= this.numOnes) {
      return null;
    }
    return this.ones[n];
  }

  // todo: needs to check for multiplicity:
  // debug_assert!(!self.has_multiplicity);
  /**
   * @param {number} n
   */
  maybeSelect0(n) {
    assert(!this.hasMultiplicity, 'cannot take select0 in the presence of multiplicity (repeated elements)');
    if (n < 0 || n >= this.numZeros) {
      return null;
    }
    // Binary search over rank0 to determine the position of the n-th 0-bit.
    const index = partitionPoint(this.universeSize, i => this.rank0(i) <= n);
    return index - 1;
  }

  /**
   * @param {number} n
   */
  select1(n) {
    const result = this.maybeSelect1(n);
    if (result === null) throw new Error(`n ${n} is not a valid 1-bit index`);
    return result;
  }

  /**
   * @param {number} n
   */
  select0(n) {
    const result = this.maybeSelect0(n);
    if (result === null) throw new Error('n is not a valid 0-bit index');
    return result;
  };

  /**
   * Get the value of the bit at the specified index (0 or 1).
   * todo: test
   * note: v inefficient
   * 
   * @param {number} index
   */
  get(index) {
    assert(index >= 0 && index <= this.universeSize);
    assert(!this.hasMultiplicity);
    const value = this.rank1(index + 1) - this.rank1(index);
    DEBUG && assert(value === 0 || value === 1);
    return value;
  }
}