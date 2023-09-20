import { assert, assertNotUndefined, assertSafeInteger, log } from "./assert.js";
import { partitionPoint } from './bits';

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

export class SortedArrayBitVec {
  /**
   * @param {number[]} ones
   * @param {number} lengthInBits
   * @param {boolean} hasMultiplicity
   */
  constructor(ones, lengthInBits, hasMultiplicity) {
    if (hasMultiplicity) {
      // assert monotonically nondecreasing
      for (let i = 1; i < ones.length; i++) assert(ones[i - 1] <= ones[i]);
    } else {
      // there cannot be more 1-bits than bits
      assert(ones.length <= length); 
      // assert monotonically increasing
      for (let i = 1; i < ones.length; i++) assert(ones[i - 1] < ones[i]);
    }
    this.ones = ones;
    this.lengthInBits = lengthInBits;
    this.numOnes = ones.length;
    this.numZeros = this.lengthInBits - this.numOnes;
    this.hasMultiplicity = hasMultiplicity;
  }

  /**
   * @param {number} index
   */
  rank1(index) {
    return partitionPoint(this.lengthInBits, i => this.ones[i] < index);
  }

  // todo: needs to check for multiplicity:
  // debug_assert!(!self.has_multiplicity);
  /**
   * @param {number} index
   */
  rank0(index) {
    if (index >= this.lengthInBits) {
      return this.numZeros;
    };
    return index - this.rank1(index);
  }

  /**
   * @param {number} n
   */
  maybeSelect1(n) {
    if (n >= this.numOnes) {
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
    if (n >= this.numZeros) {
      return null;
    }
    // return this.ones[n];
    // todo: why is this the way it is?
    const index = partitionPoint(this.lengthInBits, i => this.rank0(i) <= n);
    return index - 1;
  }






}