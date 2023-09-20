import { assert, assertNotUndefined, assertSafeInteger, log } from "./assert.js";

// todo: implement the proper bitvec interface (Eg. universe size = #ones + #zeros)

export class SortedArrayBitVec {
  /**
   * @param {number[]} ones
   * @param {number} length
   * @param {boolean} hasMultiplicity
   */
  constructor(ones, length, hasMultiplicity) {
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
    this.length = length;
    this.hasMultiplicity = hasMultiplicity;
  }

  /**
   * @param {number} index
   */
  maybeSelect1(index) {
    if (index >= this.ones.length) return null;
    return this.ones[index];
  }


}