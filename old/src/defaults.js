import { assert } from './assert.js';
import { partitionPoint } from './bits.js';

// This module provides default implementations for some bit vector functions.
// They're meant to be used only when we need a somewhat slow fallback.
// Using this approach more pervasively would cause lots of megamorphic impls, 
// so we only implement relatively slow fallback methods here, and not common 
// impls that should be fast, eg. definitions of select1 in terms of trySelect1.
// Edit: Actually, the cost of the dynamic dispatch to trySelect1 is probably
// not significant relative to the implementation of trySelect1.
//

/**
 * @param {BitVec} bv
 * @param {number} index
 */
export function rank0(bv, index) {
  assert(!bv.hasMultiplicity, 'cannot take rank0 in the presence of multiplicity (repeated elements)');
  if (index <= 0) {
    return 0;
  } else if (index >= bv.universeSize) {
    return bv.numZeros;
  };
  return index - bv.rank1(index);
}

/**
 * @param {BitVec} bv
 * @param {number} n
 */
export function trySelect0(bv, n) {
  assert(!bv.hasMultiplicity, 'cannot take select0 in the presence of multiplicity (repeated elements)');
  if (n < 0 || n >= bv.numZeros) {
    return null;
  }
  // Binary search over rank0 to determine the position of the n-th 0-bit.
  const index = partitionPoint(bv.universeSize, i => bv.rank0(i) <= n);
  return index - 1;
}

/**
 * @param {BitVec} bv
 * @param {number} n
 */
export function trySelect1(bv, n) {
  assert(!bv.hasMultiplicity, 'cannot take select1 in the presence of multiplicity (repeated elements)');
  if (n < 0 || n >= bv.numZeros) {
    return null;
  }
  // Binary search over rank1 to determine the position of the n-th 1-bit.
  const index = partitionPoint(bv.universeSize, i => bv.rank1(i) <= n);
  return index - 1;
}

/**
 * Get the value of the bit at the specified index (0 or 1).
 * Note: This is rather inefficient since it does two rank calls,
 * each of which takes O(log(n)) time.
 * 
 * In the presence of multiplicity, returns the count of the bit.
 * 
 * @param {BitVec} bv
 * @param {number} index
 */
export function get(bv, index) {
  assert(index >= 0 && index <= bv.universeSize);
  const value = bv.rank1(index + 1) - bv.rank1(index);
  if (DEBUG && !bv.hasMultiplicity) {
    assert(value === 0 || value === 1, () => `expected 0 or 1, got ${value}`);
  }
  return value; 
}

/**
* @param {BitVec} bv
* @param {number} n
*/
export function select0(bv, n) {
  const result = bv.trySelect0(n);
  if (result === null) {
    throw new Error(`n (${n}) is not a valid 0-bit index`);
  }
  return result;
};

/**
* @param {BitVec} bv
* @param {number} n
*/
export function select1(bv, n) {
  const result = bv.trySelect1(n);
  if (result === null) {
    throw new Error(`n (${n}) is not a valid 1-bit index`);
  }
  return result;
}