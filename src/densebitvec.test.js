import fc from 'fast-check';
import { describe, expect, it, test } from 'vitest';
import { DenseBitVec, DenseBitVecBuilder } from './densebitvec';
import { testBitVecType } from './testutils.js';

// - test with VERY different block sizes (not just 5 and 6)
// note: concurrency does not seem to cause tests to run at the same time
describe(`DenseBitVec in varying sampling configurations`, () => {
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