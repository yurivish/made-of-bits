import { unicodeString } from 'fast-check';
import { assert, assertNonNegative, assertSafeInteger } from './assert.js';
import * as defaults from './defaults';
import { trySelect0 } from './defaults.js';
import { bits } from './index.js';
import { ascending } from './sort.js';
import { SparseBitVec } from './sparsebitvec.js';

/**
 * @implements {BitVecBuilder}
 */
export class RLEBitVecBuilder {

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
    const builder = new RLERunBuilder();

    let prev = -1;
    for (const cur of this.ones) {
      assertNonNegative(cur);
      assertSafeInteger(cur);
      const numZeros = cur - prev - 1;
      assert(numZeros >= 0);
      builder.run(numZeros, 1);
      prev = cur;
    }

    // pad out with zeros if needed
    const numZeros = this.universeSize - Math.max(0, prev);
    builder.run(numZeros, 0);

    return builder.build(options);
  }
}

// note: does not implement BitVecBuilder since it is based on a run() method!
// todo: figure out how to handle this
export class RLERunBuilder {
  constructor() {
    /** @type number[] */
    this.z = [];

    /** @type number[] */
    this.zo = [];

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
    if (numZeros === 0 && length > 0) {
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
      this.zo.push(this.numZeros + this.numOnes);
    }
  }

  build(options = {}) {
    // todo: assert there are no options?
    // The +1 to the universe size is needed because the 1-bit marker in z
    // comes at the position after `this.numZeros` zeros, and the same idea
    // applies to zo, which marks with a 1-bit the position after each 01-run.
    const z = new SparseBitVec(this.z, this.numZeros + 1);
    const zo = new SparseBitVec(this.zo, this.numZeros + this.numOnes + 1);
    return new RLEBitVec(z, zo, this.numZeros, this.numOnes);
  }

  lastBlockContainsOnlyZeros() {
    const length = this.z.length;
    if (length === 0) {
      return false;
    } else if (length === 1) {
      return this.z[0] === this.zo[0];
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
   * @param {number} numZeros
   * @param {number} numOnes
   */
  constructor(z, zo, numZeros, numOnes) {
    /**
     * Sparse bit vector marking the cumulative number of zeros
     * after every 0-run.
     * 
     * @readonly */
    this.z = z;

    /**
     * Sparse bit vector marking the cumulative number of zeros
     * and ones after every 01-run.
     * 
     * @readonly */
    this.zo = zo;

    /** @readonly */
    this.numZeros = numZeros;

    /** @readonly */
    this.numOnes = numOnes;

    /** @readonly */
    this.universeSize = numZeros + numOnes;

    /** @readonly */
    this.hasMultiplicity = false;
    
    /** @readonly */
    this.numUniqueZeros = this.numZeros;

    /** @readonly */
    this.numUniqueOnes = this.numOnes;
  }

  /**
   * @param {number} index
   */
  rank1(index) {
    if (index <= 0) {
      return 0;
    } else if (index >= this.universeSize) {
      return this.numOnes;
    }

    // Number of complete 01-runs up to the virtual index `index`
    const j = this.zo.rank1(index);

    // Number of zeros including the j-th block
    const numCumulativeZeros = this.z.select1(j);

    // Number of zeros preceding the j-th block
    const numPrecedingZeros = this.z.trySelect1(j - 1) ?? 0;
      
    // Number of zeros in the j-th block
    const numZeros = numCumulativeZeros - numPrecedingZeros;

    // Start index of the j-th block
    const blockStart = this.zo.trySelect1(j - 1) ?? 0;

    // Number of ones preceding the j-th block
    const numPrecedingOnes = blockStart - numPrecedingZeros;

    // Start index of ones in the j-th block
    const onesStart = blockStart + numZeros;

    const adjustment = Math.max(0, index - onesStart);
    return numPrecedingOnes + adjustment;
  }

  /**
   * @param {number} n
   */
  trySelect1(n) {
    if (n < 0 || n >= this.numOnes) {
      return null;
    }

    // The n-th one is in the j-th 01-block.
    const j = bits.partitionPoint(this.z.numOnes, i => this.zo.select1(i) - this.z.select1(i) <= n);

    // Number of zeros up to and including the j-th block
    const numCumulativeZeros = this.z.select1(j);

    return numCumulativeZeros + n;
  }


  /**
   * @param {number} n
   */
  trySelect0(n) {
    if (n < 0 || n >= this.numZeros) {
      return null;
    };

    // The n-th zero is in the j-th 01-block.
    let j = this.z.rank1(n + 1);

    // If we're in the first 01-block, the n-th zero is at index n.
    if (j === 0) {
      return n;
    };

    // Start index of the j-th 01-block
    let blockStart = this.zo.select1(j - 1);

    // Number of zeros preceding the j-th 01-block
    let numPrecedingZeros = this.z.select1(j - 1);

    // Return the index of the (n - numPrecedingZeros)-th zero in the j-th 01-block.
    return blockStart + (n - numPrecedingZeros);
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
  select0(n) {
    return defaults.select0(this, n);
  };
  
  /**
   * @param {number} n
   */
  select1(n) {
    return defaults.select1(this, n);
  }

  /**
   * @param {number} index
   */
  get(index) {
    return defaults.get(this, index);
  }

}