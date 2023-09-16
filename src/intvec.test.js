import { describe, expect, it, test } from 'vitest';
import * as bits from './bits.js';
import { IntVec } from "./intvec.js";

describe('IntVec', () => { 
  it('should disallow getting an element from an empty IntVec', () => {
    expect(() => new IntVec(0, 0).get(0)).toThrow();
  });

  it('should return zero elements before anything is pushed', () => {
    const xs = new IntVec(3, 7);
    for (let i = 0; i < 3; i++) {
      expect(xs.get(i)).toBe(0);
    }
  });

  it('should throw on out-of-bounds indices (in debug mode)', () => {
    const xs = new IntVec(3, 7);
    expect(() => xs.get(-1)).toThrow();
    expect(() => xs.get(4)).toThrow();
    expect(() => xs.get(5)).toThrow();
  });

  it('should allow writing and reading elements', () => {
    const tests = [
      { bitWidth: 0, values: [0, 0, 0] },
      { bitWidth: 1, values: [1, 0, 1, 0] },
      { bitWidth: 5, values: [1, 0, 1, 0] },
      { bitWidth: 32, values: [10, 0, 31, 2 ** 32 - 1] },
    ];

    for (const { bitWidth, values } of tests) {
      const xs = new IntVec(values.length, bitWidth);

      // test value too small
      expect(() => xs.push(-1)).toThrow();
      // test value too large
      expect(() => xs.push(2 ** bitWidth)).toThrow();

      for (let i = 0; i < values.length; i++) {
        const value = values[i];
        // test the value before writing
        expect(xs.get(i)).toBe(0);
        // push the value
        xs.push(value);
        // test the value has been pushed
        expect(xs.get(i)).toBe(value);
      }

      // it should disallow getting beyond the end
      expect(() => xs.get(xs.length)).toThrow();

      // it should disallow pushing beyond the end, unless
      // the bit width is zero.
      if (bitWidth > 0) {
        expect(() => xs.push(0)).toThrow();
      } else {
        expect(() => xs.push(0)).not.toThrow();
      }
    }
  });
});
