import { describe, expect, it, test } from 'vitest';
import { DEBUG } from './assert.js';
import { BitBuf } from './bitbuf';
import { DenseBitVec } from './densebitvec';

// todo: test with varying rankselect index parameters other than (5, 5)

describe('DenseBitVec', () => {
  test('one bit set', () => {
    const lengthInBits = 123;
    const step = 1;

    const buf = new BitBuf(lengthInBits);
    for (let bitIndex = 0; bitIndex < lengthInBits; bitIndex += step) {
      if (bitIndex > 0) buf.setZero(bitIndex - step);

      buf.setOne(bitIndex);
      const bv = new DenseBitVec(buf, 5, 5);

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
      expect(bv.rank0(bitIndex + 1e6)).toBe(bv.lengthInBits - 1);

      // rank1
      expect(bv.rank1(0)).toBe(0);
      expect(bv.rank1(bitIndex)).toBe(0);
      expect(bv.rank1(bitIndex + 1)).toBe(1);
      expect(bv.rank1(bitIndex + 100)).toBe(1);
    }
  });

  test('two bits set', () => {
    const lengthInBits = 123;
    const step = 1;

    for (let bitIndex1 = 0; bitIndex1 < lengthInBits; bitIndex1 += step) {
      for (let bitIndex2 = bitIndex1 + step; bitIndex2 < lengthInBits; bitIndex2 += step) {
        const buf = new BitBuf(lengthInBits);
        buf.setOne(bitIndex1);
        buf.setOne(bitIndex2);

        const bv = new DenseBitVec(buf, 5, 5);

        // select1
        expect(bv.select1(0)).toBe(bitIndex1);
        expect(bv.select1(1)).toBe(bitIndex2);
        expect(() => bv.select1(-1)).toThrow();
        expect(() => bv.select1(2)).toThrow();

        // rank0
        // expect(bv.rank0(0)).toBe(0);
        // expect(bv.rank0(bitIndex)).toBe(bitIndex);
        // expect(bv.rank0(bitIndex + 1)).toBe(bitIndex);
        // expect(bv.rank0(bitIndex + 1e6)).toBe(bv.lengthInBits - 1);

        // rank1
        expect(bv.rank1(0)).toBe(0);
        expect(bv.rank1(bitIndex1)).toBe(0);
        expect(bv.rank1(bitIndex2)).toBe(1);
        expect(bv.rank1(bitIndex2 + 1)).toBe(2);
        expect(bv.rank1(bitIndex2 + 100)).toBe(2);
      }
    }
  });


});

// todo
// - Are there any bounds on the number of 1 bits we can store in a dense bit vector?

