import { describe, expect, it, test } from 'vitest';
import { MultiBitVecBuilder } from './multibitvec.js';
import { testBitVecType, testMultiBitVecType } from './testutils.js';

testBitVecType(MultiBitVecBuilder);
testMultiBitVecType(MultiBitVecBuilder);

describe('MultiBitVec', () => {
  test('can contain (very large) multitudes', () => {
    const builder = new MultiBitVecBuilder(6);
    builder.one(0, 1e9);
    builder.one(3, 1e9);
    builder.one(5, 1e9);
    const bv = builder.build();
    expect(bv.rank1(0)).toBe(0);
    expect(bv.rank1(1)).toBe(1e9);
    expect(bv.rank1(2)).toBe(1e9);
    expect(bv.rank1(3)).toBe(1e9);
    expect(bv.rank1(4)).toBe(2e9);
    expect(bv.rank1(5)).toBe(2e9);
    expect(bv.rank1(6)).toBe(3e9);
    expect(bv.rank1(7)).toBe(3e9);
  });
});