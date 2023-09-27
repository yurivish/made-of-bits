import { SortedArrayBitVec, SortedArrayBitVecBuilder } from './sortedarraybitvec.js';
import { testBitVecType, testMultiBitVecType } from './testutils.js';

testBitVecType(SortedArrayBitVecBuilder);


// work ongoing to add multiset testing.
// also to figure out the appropriate uniform interface for both
// and also how to supprt/not support rank0 when that is what happens... maybe tests should just .?().
testMultiBitVecType(SortedArrayBitVecBuilder);