import { describe, expect, it, test } from 'vitest';
import * as bits from './bits.js';

describe('BLOCK_BITS', () => {
  it('is a power of two', () => {
    expect(Number.isInteger(Math.log2(bits.BLOCK_BITS))).toBe(true);
  });
  it('is correctly reflected in BLOCK_BITS_LOG2', () => {
    expect(bits.BLOCK_BITS_LOG2).toBe((Math.log2(bits.BLOCK_BITS)));
  });
});

describe('u32', () => {
  it('correctly handles values in range', () => {
    expect(bits.u32(0)).toBe(0);
    expect(bits.u32(12345)).toBe(12345);
    expect(bits.u32(2 ** 32 - 1)).toBe(2 ** 32 - 1);
  });

  it('does not allow numbers outside [0, 2^32)', () => {
    expect(() => bits.u32(-12345)).toThrow();
    expect(() => bits.u32(-1)).toThrow();
    expect(() => bits.u32(2 ** 32)).toThrow();
    expect(() => bits.u32(2 ** 32 + 12345)).toThrow();
  });
});

test('bitBlockOffset', () => {
  it('returns the correct value for small inputs', () => {
    // zero should always be zero, regardless of block size
    expect(bits.blockBitOffset(0)).toBe(0);
    // values less than a block size should be returned as they are.
    expect(bits.blockBitOffset(bits.BLOCK_BITS >> 1)).toBe(bits.BLOCK_BITS >> 1);
    // multiples of the block size should be zero
    expect(bits.blockBitOffset(bits.BLOCK_BITS)).toBe(0);
  });

  it('handles being offset by a large number of blocks', () => {
    const blockIndices = [100, 12345];
    const bitOffsets = [bits.BLOCK_BITS >> 1, bits.BLOCK_BITS >> 2];
    for (const blockIndex of blockIndices) {
      for (const bitOffset of bitOffsets) {
        expect(bits.blockBitOffset(blockIndex * bits.BLOCK_BITS + bitOffset)).toBe(bitOffset);
      }
    }
  });
});

describe('oneMask', () => {
  it('returns the appropriate mask', () => {
    for (let i = 0; i < 33; i++) {
      expect(bits.oneMask(i)).toBe(2 ** i - 1);
    }
  });

  it('throws if the number is out of range', () => {
    expect(() => bits.oneMask(-1)).toThrow();
    expect(() => bits.oneMask(33)).toThrow();
  });
});

