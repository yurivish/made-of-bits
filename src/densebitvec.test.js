import { describe, expect, it, test } from 'vitest';
import { DEBUG } from './assert.js';
import { BitBuf } from './bitbuf';
import { DenseBitVec } from './densebitvec';


// todo: document why this is the chosen invariant rather than
//       making the functions inverses of each other
//       - half open interval is convenient
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
 * @param {DenseBitVec} bv
 */
function testInvariants(bv) {
  // rank1(select1(n)) === n
  for (let n = 0; n < bv.numOnes; n++) {
    expect(bv.rank1(bv.select1(n))).toBe(n);
    expect(bv.rank1(bv.select1(n) + 1)).toBe(n + 1);
  }

  // rank0(select0(n)) === n
  for (let n = 0; n < bv.numZeros; n++) {
    expect(bv.rank0(bv.select0(n))).toBe(n);
    expect(bv.rank0(bv.select0(n) + 1)).toBe(n + 1);
  }
}

/**
 * todo: have a BitVec interface or type
 * Tests a BitVec implementation for basic correctness.
 * Does not perform very sophisticated checks, since our strategy
 * is to test the simple sorted array implementation for correctness,
 * then test other BitVecs with it as the ground truth baseline.
 * @param {typeof DenseBitVec} BitVec
 */
function testBitVecType(BitVec) {
  test('one bit set', () => {
    const lengthInBits = 123;
    // save time by only testing with every `step`-th bit set
    const step = 13;

    const buf = new BitBuf(lengthInBits);
    for (let bitIndex = 0; bitIndex < lengthInBits; bitIndex += step) {
      if (bitIndex > 0) buf.setZero(bitIndex - step);

      buf.setOne(bitIndex);
      const bv = new BitVec(buf, 5, 5);
      testInvariants(bv);

      // select0
      if (bitIndex === 0) {
        expect(bv.select0(0)).toBe(1);
      } else {
        expect(bv.select0(0)).toBe(0);
        expect(bv.select0(bitIndex - 1)).toBe(bitIndex - 1);
      }

      if (bitIndex === lengthInBits - 1) {
        expect(bv.maybeSelect0(bitIndex)).toBe(null);
      } else {
        expect(bv.select0(bitIndex)).toBe(bitIndex + 1);
      }

      // select1
      expect(bv.select1(0)).toBe(bitIndex);
      expect(bv.maybeSelect1(1)).toBe(null);
      expect(() => bv.select1(-1)).toThrow();
      expect(() => bv.select1(1)).toThrow();

      // rank0
      expect(bv.rank0(0)).toBe(0);
      expect(bv.rank0(bitIndex)).toBe(bitIndex);
      expect(bv.rank0(bitIndex + 1)).toBe(bitIndex);
      expect(bv.rank0(1e6)).toBe(bv.lengthInBits - 1);

      // rank1
      expect(bv.rank1(0)).toBe(0);
      expect(bv.rank1(bitIndex)).toBe(0);
      expect(bv.rank1(bitIndex + 1)).toBe(1);
      expect(bv.rank1(bitIndex + 100)).toBe(1);
    }
  });

  test('two bits set', () => {
    const lengthInBits = 123;
    // save time by only testing with every `step`-th bit set
    const step = 13;

    for (let bitIndex1 = 0; bitIndex1 < lengthInBits; bitIndex1 += step) {
      for (let bitIndex2 = bitIndex1 + step; bitIndex2 < lengthInBits; bitIndex2 += step) {
        const buf = new BitBuf(lengthInBits);
        buf.setOne(bitIndex1);
        buf.setOne(bitIndex2);

        const bv = new BitVec(buf, 5, 5);
        testInvariants(bv);

        // select0
        // with 2 bits the edge cases are complex to express, so just test the first element
        expect(bv.select0(0)).toBe(bitIndex1 === 0 ? 1 : 0);

        // select1
        expect(bv.select1(0)).toBe(bitIndex1);
        expect(bv.select1(1)).toBe(bitIndex2);
        expect(() => bv.select1(-1)).toThrow();
        expect(() => bv.select1(2)).toThrow();

        // rank0
        expect(bv.rank0(0)).toBe(0);
        expect(bv.rank0(bitIndex1)).toBe(bitIndex1);
        expect(bv.rank0(bitIndex2)).toBe(bitIndex2 - 1);
        expect(bv.rank0(1e6)).toBe(bv.lengthInBits - 2);

        // rank1
        expect(bv.rank1(0)).toBe(0);
        expect(bv.rank1(bitIndex1)).toBe(0);
        expect(bv.rank1(bitIndex2)).toBe(1);
        expect(bv.rank1(bitIndex2 + 1)).toBe(2);
        expect(bv.rank1(bitIndex2 + 100)).toBe(2);
      }
    }
  });
}

describe('DenseBitVec', () => {
  testBitVecType(DenseBitVec);
});
