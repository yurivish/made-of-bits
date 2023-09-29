import { SortedArrayBitVec, SortedArrayBitVecBuilder } from './sortedarraybitvec.js';
import { testBitVecType, testMultiBitVecType } from './testutils.js';

testBitVecType(SortedArrayBitVecBuilder);
testMultiBitVecType(SortedArrayBitVecBuilder);

// todo: 
// - sparse-specific tests ( very long vectors, with very few bits. )
//   - In this scenario, how can we leverage the existing tests which
//   - might take a while if we test all of the spaces between the 1 bits?

