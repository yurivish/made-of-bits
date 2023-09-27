import * as d3 from 'd3';
import fc from 'fast-check';
import { describe, expect, it, test } from 'vitest';
import { assertSafeInteger } from './assert.js';
import { BitBuf } from './bitbuf';
import { DenseBitVec, DenseBitVecBuilder } from './densebitvec';
import { bits } from './index.js';
import { SortedArrayBitVec, SortedArrayBitVecBuilder } from './sortedarraybitvec.js';

// todo:
// - look into concurrent testsing (https://vitest.dev/guide/features.html)
// - idea: flipped testing – flip the bits, and then test with rank/select 0/1 reversed.
// - insight: succinct data structures can have simple implementations used as a testing baseline,
//   eg. we can check all bitvec impls against the sorted array impl.

//

/**
 * Test a specific BitVec instance in a general way, ie. for internal consistency.
 * - Checks invariants that must hold between rank & select
 * - Checks that the behaviors of `get` match `select`
 * @param {BitVec} bv
 */
export function testBitVec(bv) {
  // if non-multi-bitvec
  expect(bv.numZeros + bv.numOnes).toBe(bv.universeSize);

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

    // Verifies that rank1(select1(n)) === n
    expect(bv.rank1(select1)).toBe(n);
    expect(bv.rank1(select1 + 1)).toBe(n + 1);

    // Check `get` behavior for valid indices
    expect(bv.get(select1)).toBe(1);
  }

  for (let n = 0; n < bv.numZeros; n++) {
    const select0 = bv.select0(n);

    // Verifies that rank0(select0(n)) === n
    expect(bv.rank0(select0)).toBe(n);
    expect(bv.rank0(select0 + 1)).toBe(n + 1);

    // Check `get` behavior for valid indices
    expect(bv.get(select0)).toBe(0);
  }
}

/**
 * Returns an unsorted sample of k elements without replacement, sampled
 * from the universe [0, `n`).
 * This algorithm is best when the sample is sparse, ie. `k` is much smaller than `n`.
 * If `k` and `n` are of comparable size, then you may as well just do
 * an in-place Fischer-Yates shuffle of the full range as an array:
 *   https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle
 * 
 * The idea comes from the paper
 *
 *   - Title: Simple, Optimal Algorithms for Random Sampling Without Replacement
 *   - Link: https://arxiv.org/abs/2104.05091
 *   - Author Daniel Ting: https://stat.uw.edu/about-us/people/daniel-ting
 *
 * Note: If we were in a typed language, instead of storing the `index` in `m`, we could
 * store `n - index` (or `n - index - 1` or something) to make the numbers smaller, potentially
 * being able to use a smaller integer type for the keys and values of `m` (in a typed language).
 * 
 * @param {number} k - number of elements to sample
 * @param {number} n - universe size
 * @param { () => number } rng - random number generator function
 */
function sparseFisherYatesSample(k, n, rng) {
  assertSafeInteger(k);
  assertSafeInteger(n);
  if (n < 0) throw new Error("n must be nonnegative");
  if (k > n) throw new Error("k must not exceed n");
  if (n === 0) return new Uint32Array();
  const m = new Map();
  const xs = new Uint32Array(k);
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
 * @param {BitVecBuilderConstructable} BitVecBuilder
 * @param {object} buildOptions - options passed to the builder's `build` method
 */
export function testBitVecProperties(BitVecBuilder, buildOptions = {}) {

  // 🌶️ todo: delete!
  if (true) return;


  // Generate random bitvectors with an arbitrary density of uniformly-distributed ones
  // and run them through basic consistency checks.
  fc.assert(fc.property(
    // note: the `max` here might want to be raised for more exhaustive tests,
    // but the downside is that test begin to take longer
    fc.integer({ min: 0, max: 5e2 }), 
    // @ts-ignore because of strict mode & jsdoc interactions underlining the func args w/ squigglies
    fc.integer({ min: 0, max: 5e2 }), 
    fc.infiniteStream(fc.double({ min: 0, max: 1, maxExcluded: true }).noBias()),
    function buildAndTest(numOnes, numZeros, rngStream) {
      const rng = () => rngStream.next().value;
      const universeSize = numOnes + numZeros;
      const ones = sparseFisherYatesSample(numOnes, universeSize, rng);
      const builder = new BitVecBuilder(universeSize);
      for (const one of ones) {
        builder.one(one);
      }
      const bv = builder.build(buildOptions);
      testBitVec(bv);
      return true;
    }));
}

/**
 * @param {BitVecBuilderConstructable} BitVecBuilder
 * @param {object} buildOptions - options passed to the builder's `build` method
 */
export function testMultiBitVecType(BitVecBuilder, buildOptions = {}) {  
  // fc.assert(fc.property(

  //   fc.array(fc.integer({ min: 0, max: 1e2 }), { maxLength: 1e3 }),
  //   // @ts-ignore because of strict mode & jsdoc interactions underlining the func args w/ squigglies
  //   fc.double({ min: 0, max: 1 }),
  //   function buildAndTest(ones, pcDuplicates) {
  //     const universeSize = ones.length > 0 ? ones[ones.length - 1] : 0;
  //     const builder = new BitVecBuilder(universeSize);
  //     for (const one of ones) {
  //       builder.one(one);
  //     }
  //     const bv = builder.build(buildOptions);
  //     console.log(bv.hasMultiplicity);
  //     testBitVec(bv);
  //     return true;
  //   }));
}

/**
 * Tests a BitVec implementation for basic correctness using some specific example scenarios.
 * Does not perform very sophisticated checks, since our strategy
 * is to test the simple sorted array implementation for correctness,
 * then test other BitVecs with it as the ground truth baseline.
 * This is distinct from the property tests since here we exhaustively test a specific collection
 * of examples scenarios, so it is not quite the same.
 * @param {BitVecBuilderConstructable} BitVecBuilder
 * @param {object} buildOptions - options passed to the builder's `build` method
 */
export function testBitVecType(BitVecBuilder, buildOptions = {}) {  
  testBitVecProperties(BitVecBuilder, buildOptions);

  // large enough to span many blocks
  const universeSize = bits.BasicBlockSize * 10;
  // save time by only testing with every `step`-th bit set
  const step = (bits.BasicBlockSize >>> 1) - 1;
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
