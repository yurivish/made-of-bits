import { describe, expect, it, test } from 'vitest';
import { SparseBitVec, SparseBitVecBuilder } from './sparsebitvec.js';
import { testBitVecType, testMultiBitVecType } from './testutils.js';

// testBitVecType(SparseBitVecBuilder);
// testMultiBitVecType(SparseBitVecBuilder);

describe('SparseBitVec', () => {
  test('can be very large', () => {
    const builder = new SparseBitVecBuilder(4e9 + 4);
    builder.one(1e9);
    builder.one(2e9);
    builder.one(3e9);
    const bv = builder.build();
    expect(bv.rank1(0)).toBe(0);
    expect(bv.rank1(1e9)).toBe(0);
    expect(bv.rank1(1e9 + 1)).toBe(1);
    expect(bv.rank1(2e9)).toBe(1);
    expect(bv.rank1(2e9 + 1)).toBe(2);
    expect(bv.rank1(3e9)).toBe(2);
    expect(bv.rank1(3e9 + 1)).toBe(3);
    expect(bv.rank1(4e9)).toBe(3);

    expect(bv.select1(0)).toBe(1e9);
    expect(bv.select1(1)).toBe(2e9);
    expect(bv.select1(2)).toBe(3e9);

    expect(bv.select0(0)).toBe(0);
    expect(bv.select0(1e9 - 1)).toBe(1e9 - 1);
    expect(bv.select0(1e9)).toBe(1e9 + 1);
    expect(bv.select0(2e9)).toBe(2e9 + 2);
    expect(bv.select0(3e9)).toBe(3e9 + 3);
    expect(bv.select0(4e9)).toBe(4e9 + 3);
  });
});