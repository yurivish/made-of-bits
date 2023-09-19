import { describe, expect, it, test } from 'vitest';
import { BitBuf } from './bitbuf';
import { DenseBitVec } from './densebitvec';

describe('DenseBitVec', () => {
  describe('single bit', () => {
    const buf = new BitBuf(1000);
    buf.setOne(100);
    const bv = new DenseBitVec(buf, 5, 5);
    it('should have a single bit set', () => {

      expect(bv.select1(0)).toBe(100);
      expect(bv.maybeSelect1(1)).toBe(null);

      expect(() => bv.select1(-1)).toThrow();
      expect(() => bv.select1(1)).toThrow();

      expect(bv.rank1(0)).toBe(0);
      expect(bv.rank1(100)).toBe(0);
      expect(bv.rank1(101)).toBe(1);
      expect(bv.rank1(1001)).toBe(1);
    });
  });
  
  test('nothing at all');
});

// todo
// - Are there any bounds on the number of 1 bits we can store in a dense bit vector?

