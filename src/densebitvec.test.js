import fc from 'fast-check';
import { describe, expect, it, test } from 'vitest';
import { DenseBitVec, DenseBitVecBuilder } from './densebitvec';
import { testBitVecType } from './testutils.js';

describe('DenseBitVec (5, 5)', () => {
  testBitVecType(DenseBitVecBuilder, {
    rankSamplesPow2: 5,
    selectSamplesPow2: 5
  });
});

describe('DenseBitVec (5, 6)', () => {
  testBitVecType(DenseBitVecBuilder, {
    rankSamplesPow2: 5,
    selectSamplesPow2: 6
  });
});

describe('DenseBitVec (6, 5)', () => {
  testBitVecType(DenseBitVecBuilder, {
    rankSamplesPow2: 6,
    selectSamplesPow2: 5
  });
});