import { describe, expect, it, test } from 'vitest';
import * as bits from './bits.js';
import './debug.js';
import { IntBuf } from "./intbuf.js";

describe('IntBuf', () => { 
  if (DEBUG) {
    it('should disallow getting an element from an empty IntBuf (in debug mode)', () => {
      expect(() => new IntBuf(0, 0).get(0)).toThrow();
    });
  }

  it('should return zero elements before anything is pushed', () => {
    const xs = new IntBuf(3, 7);
    for (let i = 0; i < 3; i++) {
      expect(xs.get(i)).toBe(0);
    }
  });

  if (DEBUG) {
    it('should throw on out-of-bounds indices (in debug mode)', () => {
      const xs = new IntBuf(3, 7);
      expect(() => xs.get(-1)).toThrow();
      expect(() => xs.get(4)).toThrow();
      expect(() => xs.get(5)).toThrow();
    });
  }

  it('should allow writing and reading elements', () => {
    const tests = [
      { bitWidth: 0, values: [0, 0, 0] },
      { bitWidth: 1, values: [1, 0, 1, 0] },
      { bitWidth: 5, values: [1, 0, 1, 0] },
      { bitWidth: bits.BasicBlockSize, values: [10, 0, 31, 2 ** bits.BasicBlockSize - 1] },
    ];

    for (const { bitWidth, values } of tests) {
      const xs = new IntBuf(values.length, bitWidth);

      if (DEBUG) {
        // test value too small (in debug mode)
        expect(() => xs.push(-1)).toThrow();
        // test value too large (in debug mode)
        expect(() => xs.push(2 ** bitWidth)).toThrow();
      }

      for (let i = 0; i < values.length; i++) {
        const value = values[i];
        // test the value before writing
        expect(xs.get(i)).toBe(0);
        // push the value
        xs.push(value);
        // test the value has been pushed
        expect(xs.get(i)).toBe(value);
      }

      // it should disallow getting beyond the end (in debug mode)
      if (DEBUG) {
        expect(() => xs.get(xs.length)).toThrow();
      }

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
