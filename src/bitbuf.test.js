import { describe, expect, it, test } from 'vitest';
import { BitBuf } from './bitbuf.js';

describe('BitBuf', () => {
  /**
   * Test the given BitBuf by setting and un-setting 3 bits at indices offset + 0/1/2.
   * 
   * @param {BitBuf} buf - the BitBuf to test
   * @param {number} offset - the offset within the BitBuf which to treat as zero.
   */
  function check(buf, offset) {
    it('should be initialized to false', () => {
      expect(buf.get(offset + 0)).toBe(0);
      expect(buf.get(offset + 1)).toBe(0);
      expect(buf.get(offset + 2)).toBe(0);
    });

    it('should set and un-set bits individually', () => {
      buf.setOne(offset + 1);
      expect(buf.get(offset + 0)).toBe(0);
      expect(buf.get(offset + 1)).toBe(1);
      expect(buf.get(offset + 2)).toBe(0);
      {
        const zp = buf.maybeZeroPadded();
        expect(zp.get(offset + 0)).toBe(0);
        expect(zp.get(offset + 1)).toBe(1);
        expect(zp.get(offset + 2)).toBe(0);
      }
      

      buf.setOne(offset + 2);
      expect(buf.get(offset + 0)).toBe(0);
      expect(buf.get(offset + 1)).toBe(1);
      expect(buf.get(offset + 2)).toBe(1);
      {
        const zp = buf.maybeZeroPadded();
        expect(buf.get(offset + 0)).toBe(0);
        expect(buf.get(offset + 1)).toBe(1);
        expect(buf.get(offset + 2)).toBe(1);
      }

      buf.setOne(offset + 0);
      expect(buf.get(offset + 0)).toBe(1);
      expect(buf.get(offset + 1)).toBe(1);
      expect(buf.get(offset + 2)).toBe(1);
      {
        const zp = buf.maybeZeroPadded();
        expect(zp.get(offset + 0)).toBe(1);
        expect(zp.get(offset + 1)).toBe(1);
        expect(zp.get(offset + 2)).toBe(1);
      }

      buf.setZero(offset + 2);
      expect(buf.get(offset + 0)).toBe(1);
      expect(buf.get(offset + 1)).toBe(1);
      expect(buf.get(offset + 2)).toBe(0);
      {
        const zp = buf.maybeZeroPadded();
        expect(buf.get(offset + 0)).toBe(1);
        expect(buf.get(offset + 1)).toBe(1);
        expect(buf.get(offset + 2)).toBe(0);
      }

      buf.setZero(offset + 0);
      expect(buf.get(offset + 0)).toBe(0);
      expect(buf.get(offset + 1)).toBe(1);
      expect(buf.get(offset + 2)).toBe(0);
      {
        const zp = buf.maybeZeroPadded();
        expect(buf.get(offset + 0)).toBe(0);
        expect(buf.get(offset + 1)).toBe(1);
        expect(buf.get(offset + 2)).toBe(0);
      }

      buf.setZero(offset + 1);
      expect(buf.get(offset + 0)).toBe(0);
      expect(buf.get(offset + 1)).toBe(0);
      expect(buf.get(offset + 2)).toBe(0);
      {
        const zp = buf.maybeZeroPadded();
        expect(buf.get(offset + 0)).toBe(0);
        expect(buf.get(offset + 1)).toBe(0);
        expect(buf.get(offset + 2)).toBe(0);
      }
    });

    if (DEBUG) {
      it('should throw errors if the index is out of bounds.', () => {
        expect(() => buf.get(-2)).toThrow();
        expect(() => buf.get(-1)).toThrow();
        expect(() => buf.get(buf.universeSize)).toThrow();
        expect(() => buf.get(buf.universeSize + 1)).toThrow();

        const zp = buf.maybeZeroPadded();
        expect(() => zp.get(-2)).toThrow();
        expect(() => zp.get(-1)).toThrow();
        expect(() => zp.get(zp.universeSize)).toThrow();
        expect(() => zp.get(buf.universeSize + 1)).toThrow();
      });
    }
  }

  check(new BitBuf(3), 0);
  check(new BitBuf(5), 2);

  check(new BitBuf(300), 0);
  check(new BitBuf(300), 100);
  
  check(new BitBuf(300), 0);
  check(new BitBuf(300), 100);

  it('zero-pads to the leftmost and rightmost one', () => {
    const buf = new BitBuf(123456);
    buf.setOne(0 * 32000);
    buf.setOne(0.5 * 32000);
    buf.setOne(1 * 32000 - 1);
    const zp = buf.maybeZeroPadded();
    expect(zp.blocks.length).toBe(1000);
  });
});
