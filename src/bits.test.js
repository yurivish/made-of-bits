import { describe, expect, it, test } from 'vitest';
import * as bits from './bits.js';
import './debug.js';

describe('BLOCKSIZE', () => {
  it('is a power of two', () => {
    expect(Number.isInteger(Math.log2(bits.BasicBlockSize))).toBe(true);
  });
  it('is correctly reflected in BLOCKSIZE', () => {
    expect(bits.BasicBlockSizePow2).toBe((Math.log2(bits.BasicBlockSize)));
  });
});

describe('u32', () => {
  it('correctly handles values in range', () => {
    expect(bits.u32(0)).toBe(0);
    expect(bits.u32(12345)).toBe(12345);
    expect(bits.u32(2 ** 32 - 1)).toBe(2 ** 32 - 1);
  });

  it('does not allow numbers outside [0, 2^32)', () => {
    DEBUG && expect(() => bits.u32(-12345)).toThrow();
    DEBUG && expect(() => bits.u32(-1)).toThrow();
    DEBUG && expect(() => bits.u32(2 ** 32)).toThrow();
    DEBUG && expect(() => bits.u32(2 ** 32 + 12345)).toThrow();
  });
});

test('bitBlockOffset', () => {
  it('returns the correct value for small inputs', () => {
    // zero should always be zero, regardless of block size
    expect(bits.basicBlockBitOffset(0)).toBe(0);
    // values less than a block size should be returned as they are.
    expect(bits.basicBlockBitOffset(bits.BasicBlockSize >> 1)).toBe(bits.BasicBlockSize >> 1);
    // multiples of the block size should be zero
    expect(bits.basicBlockBitOffset(bits.BasicBlockSize)).toBe(0);
  });

  it('handles being offset by a large number of blocks', () => {
    const blockIndices = [100, 12345];
    const bitOffsets = [bits.BasicBlockSize >> 1, bits.BasicBlockSize >> 2];
    for (const blockIndex of blockIndices) {
      for (const bitOffset of bitOffsets) {
        expect(bits.basicBlockBitOffset(blockIndex * bits.BasicBlockSize + bitOffset)).toBe(bitOffset);
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
    DEBUG && expect(() => bits.oneMask(-1)).toThrow();
    DEBUG && expect(() => bits.oneMask(33)).toThrow();
  });
});

describe('popcount', () => {
  it('returns the correct value for small numbers', () => {
    expect(bits.popcount(0)).toBe(0);
    expect(bits.popcount(0b0001)).toBe(1);
    expect(bits.popcount(0b0010)).toBe(1);
    expect(bits.popcount(0b0011)).toBe(2);
    expect(bits.popcount(0b0100)).toBe(1);
    expect(bits.popcount(0b0101)).toBe(2);
    expect(bits.popcount(0b0110)).toBe(2);
    expect(bits.popcount(0b0111)).toBe(3);
    expect(bits.popcount(0b1000)).toBe(1);
  });

  it('returns the correct value for large numbers', () => {
    expect(bits.popcount(0b11111111111111111111111111111111)).toBe(32);
    expect(bits.popcount(0b11111111111111111111111111111110)).toBe(31);
    expect(bits.popcount(0b11111111111111111111111111111101)).toBe(31);
    expect(bits.popcount(0b11111111111111111111111111111100)).toBe(30);
    expect(bits.popcount(0b11111111111111111111111111111011)).toBe(31);
    expect(bits.popcount(0b11111111111111111111111111111010)).toBe(30);
    expect(bits.popcount(0b11111111111111111111111111111001)).toBe(30);
    expect(bits.popcount(0b11111111111111111111111111111000)).toBe(29);
    DEBUG && expect(() => bits.popcount(2 ** 32)).toThrow();
  });
});

describe('trailing0', () => {
  it('returns the correct value for small numbers', () => {
    expect(bits.trailing0(0)).toBe(32);
    expect(bits.trailing0(0b01111)).toBe(0);
    expect(bits.trailing0(0b01110)).toBe(1);
    expect(bits.trailing0(0b01111)).toBe(0);
    expect(bits.trailing0(0b01100)).toBe(2);
    expect(bits.trailing0(0b01101)).toBe(0);
    expect(bits.trailing0(0b01110)).toBe(1);
    expect(bits.trailing0(0b01111)).toBe(0);
    expect(bits.trailing0(0b01000)).toBe(3);
  });

  it('returns the correct value for large numbers', () => {
    expect(bits.trailing0(0b11111111111111111111111111111110)).toBe(1);
    expect(bits.trailing0(0b11111111111111111111111111111111)).toBe(0);
    expect(bits.trailing0(0b11111111111111111111111111111100)).toBe(2);
    expect(bits.trailing0(0b11111111111111111111111111111101)).toBe(0);
    expect(bits.trailing0(0b11111111111111111111111111111110)).toBe(1);
    expect(bits.trailing0(0b11111111111111111111111111111111)).toBe(0);
    expect(bits.trailing0(0b11111111111111111111111111111000)).toBe(3);
    DEBUG && expect(() => bits.trailing0(2 ** 32)).toThrow();
  });
});

describe('bitFloor', () => {
  it('returns the correct value for small numbers', () => {
    expect(bits.bitFloor(0)).toBe(0);
    expect(bits.bitFloor(1)).toBe(1);
    expect(bits.bitFloor(2)).toBe(2);
    expect(bits.bitFloor(3)).toBe(2);
    expect(bits.bitFloor(4)).toBe(4);
    expect(bits.bitFloor(5)).toBe(4);
    expect(bits.bitFloor(6)).toBe(4);
    expect(bits.bitFloor(7)).toBe(4);
    expect(bits.bitFloor(8)).toBe(8);
  });

  it('returns the correct value for large numbers', () => {
    expect(bits.bitFloor(2 ** 31 - 2)).toBe(2 ** 30);
    expect(bits.bitFloor(2 ** 31 - 1)).toBe(2 ** 30);
    expect(bits.bitFloor(2 ** 31 - 0)).toBe(2 ** 31);
    expect(bits.bitFloor(2 ** 32 - 1)).toBe(2 ** 31);
    DEBUG && expect(() => bits.bitFloor(2 ** 32)).toThrow();
  });
});

describe('partitionPoint', () => {
  it('returns 0 if the predicate is always false', () => {
    expect(bits.partitionPoint(0, () => false)).toBe(0);
    expect(bits.partitionPoint(1, () => false)).toBe(0);
    expect(bits.partitionPoint(10, () => false)).toBe(0);
  });

  it('returns n if the predicate is always true', () => {
    expect(bits.partitionPoint(0, () => true)).toBe(0);
    expect(bits.partitionPoint(1, () => true)).toBe(1);
    expect(bits.partitionPoint(10, () => true)).toBe(10);
  });


  it('returns the split point for small numbers', () => {
    const n = 10;
    for (let split = 0; split < n; split++) {
      expect(bits.partitionPoint(n, (i) => i < split)).toBe(split);
    }
  });

  it('returns the split point for large numbers', () => {
    const n = 2 ** 32 - 1;
    expect(bits.partitionPoint(n, (i) => i < 2 ** 15)).toBe(2 ** 15);
    expect(bits.partitionPoint(n, (i) => i < n)).toBe(n);
  });
  
  if (DEBUG) {
    it('errors for numbers exceeding 32 bits (in debug mode)', () => {
      expect(() => bits.partitionPoint(2 ** 40, (i) => i < 2 ** 35)).toThrow();
    });
  }
});

describe('select1', () => {
  it('returns 32 for a non-existent bit', () => {
    expect(bits.select1(0, 0)).toBe(32);
    expect(bits.select1(0b11111, 6)).toBe(32);
    expect(bits.select1(0b11111, 7)).toBe(32);
    expect(bits.select1(0b11111, 12345)).toBe(32);
  });

  it('returns the index of the k-th bit (from the LSB up)', () => {
    const n = 0b0111000110010;
    expect(bits.select1(n, 0)).toBe(1);
    expect(bits.select1(n, 1)).toBe(4);
    expect(bits.select1(n, 2)).toBe(5);
    expect(bits.select1(n, 3)).toBe(9);
    expect(bits.select1(n, 4)).toBe(10);
    expect(bits.select1(n, 5)).toBe(11);
    expect(bits.select1(n, 6)).toBe(32);
  });

  if (DEBUG) {
    it('errors for numbers exceeding 32 bits (in debug mode)', () => {
      expect(() => bits.select1(2 ** 32, 0)).toThrow();
    });
  }
});