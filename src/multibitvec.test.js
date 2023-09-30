import { describe, expect, it, test } from 'vitest';
import { MultiBitVecBuilder } from './multibitvec.js';
import { testBitVecType, testMultiBitVecType } from './testutils.js';

testBitVecType(MultiBitVecBuilder);
testMultiBitVecType(MultiBitVecBuilder);

describe('MultiBitVec', () => {
  const builder = new MultiBitVecBuilder(100);
  test('foo', () => 0);
});