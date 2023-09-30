import { assert, assertSafeInteger } from './assert.js';
import { BitBuf } from './bitbuf.js';
import * as defaults from './defaults';
import { DenseBitVec } from './densebitvec.js';
import { ascending } from './sort.js';
import { SparseBitVec } from './sparsebitvec.js';

/**
 * 
 * @implements {BitVecBuilder}
 */
export class MultiBitVecBuilder {
  /**
   * @param {number} universeSize
   */
  constructor(universeSize) {
    this.buf = new BitBuf(universeSize);
    /**
     * Stores a map from 1-bit index to its multiplicity (count).
     * @type Map<number, number> */
    this.counts = new Map();
  }

  /**
   * @param {number} index
   */
  one(index, count = 1) {
    assert(count > 0);
    assertSafeInteger(count);
    this.buf.setOne(index);
    this.counts.set(index, (this.counts.get(index) ?? 0) + count);
  }

  build({ occupancyRank1SamplesPow2 = 10, occupancySelectSamplesPow2 = 10 } = {}) {
    console.log();
    // sort 
    const entries = Array.from(this.counts.entries()).sort((a, b) => ascending(a[0], b[0]));
    const cumulativeCounts = new Float64Array(entries.map(kv => kv[1]));
    const len = cumulativeCounts.length;
    for (let i = 1; i < len; i++) {
      cumulativeCounts[i] += cumulativeCounts[i - 1];
    }

    const occupancy = new DenseBitVec(this.buf, occupancyRank1SamplesPow2, occupancySelectSamplesPow2);
    const multiplicity = new SparseBitVec(cumulativeCounts, len > 0 ? cumulativeCounts[len - 1] + 1 : 0);
    return new MultiBitVec(occupancy, multiplicity);
  }
}

/**
 * This is a bitvec that encodes multiplicity explicitly,
 * rather than via repetition. 
 * 
 * Only 1-bits are allowed to be repeated.
 * 
 * Unlike many other multiplicity-capable structures, this one allows rank0/select0.
 * 
 * Maybe there's a better name.
 * 
 * This is effectively the histogram, minus index<>bin translation.
 *
 * @implements {BitVec}
 */
export class MultiBitVec {
  /**
   * @param {BitVec} occupancy - bitset with a 1 at every position where the count is greater than zero
   * @param {BitVec} multiplicity - for every set 1-bit in `occupancy`, contains the cumulative count up to and including that bit position.
   */
  constructor(occupancy, multiplicity) {
    this.occupancy = occupancy;
    this.multiplicity = multiplicity;

    // The number of ones represented by this bit vector is the largest set bit in multiplicity.
    this.numOnes = multiplicity.numOnes === 0 ? 0 : multiplicity.select1(multiplicity.numOnes - 1);
    this.numZeros = occupancy.numZeros;

    this.numUniqueOnes = this.occupancy.numOnes;
    this.numUniqueZeros = this.numZeros;

    this.universeSize = occupancy.universeSize;

    this.hasMultiplicity = this.numOnes > this.numUniqueOnes;
  }

  /**
   * @param {number} index
   */
  rank1(index) {
    const n = this.occupancy.rank1(index);
    if (n === 0) {
      return 0;
    } else {
      return this.multiplicity.select1(n - 1);
    }
  }

  /**
   * @param {number} n
   */
  trySelect1(n) {
    // We need this check here because rank1 returns 0 if its argument is negative.
    if (n < 0) {
      return null;
    }
    const i = this.multiplicity.rank1(n + 1);
    return this.occupancy.trySelect1(i);
  }

  /**
   * @param {number} n
   */
  trySelect0(n) {
    return this.occupancy.trySelect0(n);
  }

  /**
   * @param {number} index
   */
  rank0(index) {
    return this.occupancy.rank0(index);
  }

  /**
   * @param {number} n
   * @returns {number}
   */
  select0(n) {
    return defaults.select0(this, n);
  };
  
  /**
   * @param {number} n
   * @returns {number}
   */
  select1(n) {
    return defaults.select1(this, n);
  }

  /**
   * @param {number} index
   */
  get(index) {
    return defaults.get(this, index);
    // return this.occupancy.get(index);
  }
}