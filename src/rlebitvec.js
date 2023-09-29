import { assert } from './assert.js';
import { SparseBitVec } from './sparsebitvec.js';

// pad out the vector to the desired universe size if needed
const diff = this.universeSize - (this.numOnes + this.numZeros);
assert(diff >= 0);
if (diff > 0) {
  this.run(diff, 0);
}


/**
 * @implements {BitVecBuilder}
 */

// note: does not implement BitVecBuilder since it is based on a run() method!
export class RLERunBitVectorBuilder {

  /**
   * @param {number} universeSize
   */
  constructor() {
    /** @type number[] */
    this.z = [];

    /** @type number[] */
    this.zo = [];

    this.length = 0;
    this.numZeros = 0;
    this.numOnes = 0;
  }

  /**
   * @param {number} numZeros
   * @param {number} numOnes
   */
  run(numZeros, numOnes) {
    if (numZeros === 0 && numOnes === 0) { 
      return;
    }
    const length = this.z.length;
    this.numZeros += numZeros;
    this.numOnes += numOnes;
    if (numZeros === 0 && 0 < length) {
      // this run consists of only ones; coalesce it with the
      // previous run (since all runs contain ones at their end).
      this.zo[length - 1] += numOnes;
    } else if (numOnes === 0 && this.lastBlockContainsOnlyZeros()) {
      // this run consists of only zeros; coalesce it with the
      // previous run (since it turns out to consist of only zeros).
      this.z[length - 1] += numZeros;
      this.zo[length - 1] += numZeros;
    } else {
      // No coalescing is possible; create a new block of runs.
      // Append the cumulative number of zeros to the Z array
      this.z.push(this.numZeros);
      // Append the cumulative number of ones and zeros to the ZO array
      this.zo.push(this.length);
    }
  }

  build(options = {}) {

    // todo: i don't think these +1 are needed; test this theory.
    const z = new SparseBitVec(this.z, this.numZeros + 1);
    const zo = new SparseBitVec(this.zo, this.numZeros + this.numOnes + 1);
    return new RLEBitVec(z, zo, length, this.numZeros, this.numOnes);
  }

  lastBlockContainsOnlyZeros() {
    const length = this.z.length;
    if (length === 0) {
      return false;
    } else if (length === 1) {
      return this.z[0] - this.zo[0];
    } else {
      const lastBlockLength = this.zo[length - 1] - this.zo[length - 2];
      const lastBlockNumZeros = this.z[length - 1] - this.z[length - 2];
      return lastBlockLength === lastBlockNumZeros;
    }
  }
}

/**
 * @implements {BitVec}
 */
export class RLEBitVec {

  /**
   * @param {SparseBitVec} z
   * @param {SparseBitVec} zo
   * @param {number} length
   * @param {number} numZeros
   * @param {number} numOnes
   */
  constructor(z, zo, length, numZeros, numOnes) {

    /** @readonly */
    this.z = z;

    /** @readonly */
    this.zo = zo;

    /** @readonly */
    this.length = length;

    /** @readonly */
    this.numZeros = numZeros;

    /** @readonly */
    this.numOnes = numOnes;

    /** @readonly */
    this.universeSize = this.numOnes + this.numZeros;

    /** @readonly */
    this.hasMultiplicity = false;

    /** @readonly */
    this.numUniqueOnes = this.numOnes;
    
    /** @readonly */
    this.numUniqueZeros = this.numZeros;

  }

  /**
 * @param {number[]} ones
 * @param {number} universeSize
 */
  // ones, universeSize


  static fromBuilder() {

  }
}