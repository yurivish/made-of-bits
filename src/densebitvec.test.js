import fc from 'fast-check';
import { describe, expect, it, test } from 'vitest';
import { DenseBitVec, DenseBitVecBuilder } from './densebitvec';
import { testBitVecType } from './testutils.js';

// - test with VERY different block sizes (not just 5 and 6)
// note: concurrency does not seem to cause tests to run at the same time
describe('DenseBitVec in varying sampling configurations', () => {
  for (const rankSamplesPow2 of [5, 7, 10]) {
    for (const selectSamplesPow2 of [5, 7, 10]) {
      test(`(${rankSamplesPow2}, ${selectSamplesPow2})`, () => {
        testBitVecType(DenseBitVecBuilder, {
          rankSamplesPow2,
          selectSamplesPow2
        });
      });
    }
  }
});

describe('DenseBitVec batch', () => {
  test('woop', () => {
    const builder = new DenseBitVecBuilder(20000);
    builder.one(3);
    builder.one(7);
    builder.one(8);

    builder.one(3000);
    builder.one(7000);
    builder.one(8000);
    const input = [1, 3, 4, 10, 10, 4000, 12334];
    const output = new Array(input.length);
    const bv = builder.build();
    bv.rank1Batch(input, output);

    for (let i = 0; i < output.length; i++) {
      console.log('wat', i, '=>', bv.rank1(input[i]), output[i]);
      expect(bv.rank1(input[i])).toEqual(output[i]);
    }
  });
});