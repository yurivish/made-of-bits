import { SparseBitVec, SparseBitVecBuilder } from './sparsebitvec.js';
import { testBitVecType, testMultiBitVecType } from './testutils.js';

testBitVecType(SparseBitVecBuilder);
testMultiBitVecType(SparseBitVecBuilder);