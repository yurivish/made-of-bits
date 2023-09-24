import fc from 'fast-check';
import { describe, expect, it, test } from 'vitest';
import { DenseBitVec, DenseBitVecBuilder } from './densebitvec';
import { testBitVecType } from './testutils.js';

for (const rankSamplesPow2 of [5, 6]) {
  for (const selectSamplesPow2 of [5, 6]) {
    describe(`DenseBitVec (${rankSamplesPow2}, ${selectSamplesPow2})`, () => {
      testBitVecType(DenseBitVecBuilder, {
        rankSamplesPow2,
        selectSamplesPow2
      });
    });
  }
}