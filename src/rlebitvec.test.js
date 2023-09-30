import { describe, expect, it, test } from 'vitest';
import { RLEBitVec, RLEBitVecBuilder, RLERunBuilder } from './rlebitvec.js';
import { testBitVecType } from './testutils.js';

testBitVecType(RLEBitVecBuilder);

describe('RLEBitVec', () => {
  test('can handle large runs', () => {

    // Test scenario:
    // ┌──────────────┬──────────────┬──────────────┬─────────────────┐
    // │  1e9 0-bits  │  1e9 1-bits  │  1e9 0-bits  │  1e9+1e6 1-bits │
    // └──────────────┴──────────────┴──────────────┴─────────────────┘

    const builder = new RLERunBuilder();
    builder.run(1e9, 1e9);
    builder.run(1e9, 0);
    builder.run(0, 1e9);
    builder.run(0, 1e6);

    const bv = builder.build();
    expect(bv.rank1(0)).toBe(0);
    expect(bv.rank1(1e9)).toBe(0);
    expect(bv.rank1(1e9 + 1)).toBe(1);
    expect(bv.rank1(2e9)).toBe(1e9);
    expect(bv.rank1(3e9)).toBe(1e9);
    expect(bv.rank1(4e9)).toBe(2e9);
    expect(bv.rank1(4e9 + 1e6)).toBe(2e9 + 1e6);

    expect(bv.select0(1e9 - 1)).toBe(1e9 - 1);
    expect(bv.select0(1e9)).toBe(2e9);
    expect(bv.select1(2e9)).toBe(4e9);
  });
});