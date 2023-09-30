import * as defaults from './defaults';

// TODO: This type is a multiset but allows rank0; we need to
// updated our tests to handle this case.

/**
 * This is a bitvec that encodes multiplicity explicitly,
 * rather than via repetition. 
 * 
 * Only 1-bits are allowed to be repeated.
 * 
 * Maybe there's a better name.
 */
class MultiBitVec {
  /**
   * @param {BitVec} occupancy
   * @param {BitVec} multiplicity
   */
  constructor(occupancy, multiplicity) {
    this.occupancy = occupancy;
    this.multiplicity = multiplicity;

    // todo: all the standard fields
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
    return this.occupancy.get(index);
  }

}