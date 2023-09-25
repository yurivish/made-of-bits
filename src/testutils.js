import { describe, expect, it, test } from 'vitest';
import { BitBuf } from './bitbuf';
import { DenseBitVec, DenseBitVecBuilder } from './densebitvec';
import { SortedArrayBitVec, SortedArrayBitVecBuilder } from './sortedarraybitvec.js';
//       - it is convenient for select(0) to behave like indexing

// todo:
// - test with varying rankselect index parameters other than (5, 5)
// - exercise the block structure, ie. set more than bits.BLOCK_BITS ones...
// - create a BitVec interface and test the interface (port the test below to the interface)

// idea: flipped testing – flip the bits, and then test with rank/select 0/1 reversed.
// insight: succinct data structures can have simple implementations used as a testing baseline.
// also, w/ multiple implementations, can effectively test for agreement between bitvec impls.
// (equivalent to checking that they all match the sorted array impl)

//

/**
 * Test a specific BitVec instance in a general way, ie. for internal consistency.
 * - Checks invariants that must hold between rank & select
 * - Checks that the behaviors of `get` match `select`
 * @param {BitVec} bv
 */
export function testBitVec(bv) {
  expect(bv.rank1(-1)).toBe(0);
  expect(bv.rank1(0)).toBe(0);
  expect(bv.rank1(bv.universeSize + 1)).toBe(bv.numOnes);

  expect(bv.rank0(-1)).toBe(0);
  expect(bv.rank0(0)).toBe(0);
  expect(bv.rank0(bv.universeSize + 1)).toBe(bv.numZeros);

  expect(() => bv.select0(-1)).toThrow();
  expect(() => bv.select0(bv.universeSize + 1)).toThrow();
  
  expect(() => bv.select1(-1)).toThrow();
  expect(() => bv.select1(bv.universeSize + 1)).toThrow();

  expect(() => bv.get(-1)).toThrow();
  expect(() => bv.get(bv.universeSize + 1)).toThrow();

  expect(() => bv.get(-1)).toThrow();
  expect(() => bv.get(bv.universeSize + 1)).toThrow();


  for (let n = 0; n < bv.numOnes; n++) {
    const select1 = bv.select1(n);

    // Check `get` behavior for valid indices
    expect(bv.get(select1)).toBe(1);

    // Verifies that rank1(select1(n)) === n
    expect(bv.rank1(select1)).toBe(n);
    expect(bv.rank1(select1 + 1)).toBe(n + 1);
  }

  for (let n = 0; n < bv.numZeros; n++) {
    const select0 = bv.select0(n);

    // Check `get` behavior for valid indices
    expect(bv.get(select0)).toBe(0);

    // Verifies that rank0(select0(n)) === n
    expect(bv.rank0(select0)).toBe(n);
    expect(bv.rank0(select0 + 1)).toBe(n + 1);
  }
}

// returns an unordered sample of k elements from the universe [0..n-1]
// sparse fisher-yates sample
// note: instead of storing the index in m, we could store n - index (or n - index - 1) or something to make the numbers smaller
// and potentially be able to use a smaller integer type for the keys and values of m (if we use a typed language).
/**
 * @param {number} k
 * @param {number} n
 */
function fisherYatesSample(k, n, rng = Math.random) {
  if (n < 0) throw new Error("n must be nonnegative");
  if (k > n) throw new Error("k must not exceed n");
  if (n === 0) return [];
  const m = new Map();
  const xs = new Float64Array(k);
  for (let i = 0; i < k; i++) {
    // iterate through the conceptual array of n elements backwards
    const index = n - i - 1;
    // random number between 0 and index
    const x = Math.floor((index + 1) * rng());
    xs[i] = m.get(x) ?? x;
    m.set(x, m.get(index) ?? index);
    if (x === index) m.delete(index);
  }
  return xs;
}

/**
 * * @param {BitVecBuilderConstructable} BitVecBuilder
 * @param {object} buildOptions - options passed to the builder's `build` method
 */
export function testBitVecProperties(BitVecBuilder, buildOptions = {}) {
  fc.assert(fc.property(
    fc.integer({ min: 0, max: 1e3 }), 
    fc.integer({ min: 0, max: 1e3 }), 
    function (/** any */ numOnes, /** any */ numZeros) {
      const universeSize = numOnes + numZeros;
      const ones = fisherYatesSample(numOnes, universeSize);
      const builder = new BitVecBuilder(universeSize);
      for (const one of ones) {
        builder.one(one);
      }
      const bv = builder.build(buildOptions);
      testBitVec(bv);
      return true;
    }
  ));
}

/**
 * Tests a BitVec implementation for basic correctness using some specific example scenarios.
 * Does not perform very sophisticated checks, since our strategy
 * is to test the simple sorted array implementation for correctness,
 * then test other BitVecs with it as the ground truth baseline.
 * @param {BitVecBuilderConstructable} BitVecBuilder
 * @param {object} buildOptions - options passed to the builder's `build` method
 */
export function testBitVecType(BitVecBuilder, buildOptions = {}) {
  testBitVecProperties(BitVecBuilder, buildOptions);

  // large enough to span many blocks
  const universeSize = 1021;
  // save time by only testing with every `step`-th bit set
  const step = 234;
  test('one bit set', () => {
    for (let bitIndex = 0; bitIndex < universeSize; bitIndex += step) {
      const builder = new BitVecBuilder(universeSize);
      builder.one(bitIndex);
      const bv = builder.build(buildOptions);
      testBitVec(bv);

      // rank0
      expect(bv.rank0(0)).toBe(0);
      expect(bv.rank0(bitIndex)).toBe(bitIndex);
      expect(bv.rank0(bitIndex + 1)).toBe(bitIndex);
      expect(bv.rank0(1e6)).toBe(bv.universeSize - 1);

      // rank1
      expect(bv.rank1(0)).toBe(0);
      expect(bv.rank1(bitIndex)).toBe(0);
      expect(bv.rank1(bitIndex + 1)).toBe(1);
      expect(bv.rank1(bitIndex + 1e6)).toBe(1);

      // select0
      if (bitIndex === 0) {
        expect(bv.select0(0)).toBe(1);
      } else {
        expect(bv.select0(0)).toBe(0);
        expect(bv.select0(bitIndex - 1)).toBe(bitIndex - 1);
      }

      if (bitIndex === universeSize - 1) {
        expect(bv.maybeSelect0(bitIndex)).toBe(null);
      } else {
        expect(bv.select0(bitIndex)).toBe(bitIndex + 1);
      }

      // select1
      expect(bv.select1(0)).toBe(bitIndex);
      expect(bv.maybeSelect1(1)).toBe(null);
      expect(() => bv.select1(-1)).toThrow();
      expect(() => bv.select1(1)).toThrow();

    }
  });

  test('two bits set', () => {
    for (let bitIndex1 = 0; bitIndex1 < universeSize; bitIndex1 += step) {
      for (let bitIndex2 = bitIndex1 + step; bitIndex2 < universeSize; bitIndex2 += step) {
        const builder = new BitVecBuilder(universeSize);
        builder.one(bitIndex1);
        builder.one(bitIndex2);
        const bv = builder.build(buildOptions);
        testBitVec(bv);

        // rank0
        expect(bv.rank0(0)).toBe(0);
        expect(bv.rank0(bitIndex1)).toBe(bitIndex1);
        expect(bv.rank0(bitIndex2)).toBe(bitIndex2 - 1);
        expect(bv.rank0(1e6)).toBe(bv.universeSize - 2);

        // rank1
        expect(bv.rank1(0)).toBe(0);
        expect(bv.rank1(bitIndex1)).toBe(0);
        expect(bv.rank1(bitIndex2)).toBe(1);
        expect(bv.rank1(bitIndex2 + 1)).toBe(2);
        expect(bv.rank1(bitIndex2 + 1e6)).toBe(2);

        // select0
        // with 2 bits the edge cases are complex to express, so just test the first element
        expect(bv.select0(0)).toBe(bitIndex1 === 0 ? 1 : 0);

        // select1
        expect(bv.select1(0)).toBe(bitIndex1);
        expect(bv.select1(1)).toBe(bitIndex2);
        expect(() => bv.select1(-1)).toThrow();
        expect(() => bv.select1(2)).toThrow();
      }
    }
  });
}
