import * as defaults from './defaults';

// todo: This type is a multiset but allows rank0; we need to
// update our tests to handle this case.

// todo: test

/**
 * This is a bitvec that encodes multiplicity explicitly,
 * rather than via repetition. 
 * 
 * Only 1-bits are allowed to be repeated.
 * 
 * Maybe there's a better name.
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

    // todo: formally implement the bitvec interface
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
    return this.occupancy.get(index);
  }

}