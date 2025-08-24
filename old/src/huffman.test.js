import { describe, expect, it, test } from 'vitest';
import * as bits from './bits.js';
import './debug.js';
import { huffmanCodeLengths, waveletMatrixCodes } from './huffman.js';

// To do
// [] add many more tests; the below are highly incomplete,
//    ported from the original notebook where development occurred.
//    See https://observablehq.com/d/346a937082292c9d
// [] test `canonicalHuffmanCodes`

describe('waveletMatrixCodes', () => {
  it('computes the expected value on a simple test', () => {
    const lens = [2, 2, 2, 3];
    expect(waveletMatrixCodes(lens).map((d, i) =>
      d.toString(2).padStart(lens[i], "0")
    )).toStrictEqual(["10", "01", "11", "001"]);
  });


  it('computes the expected value on a more elaborate test', () => {
    const weights = [20, 17, 6, 3, 2, 2, 2, 1, 1, 1];
    const l = huffmanCodeLengths(weights);

    expect(l).toStrictEqual([1, 2, 4, 5, 5, 5, 5, 5, 6, 6]);

    const L = l[l.length - 1];

    const codes = waveletMatrixCodes(l);
    expect(codes).toStrictEqual([1, 1, 3, 4, 2, 1, 5, 3, 0, 1]);

    // pad with trailing 1-bits so that all codes are the same length and can be inserted into the wavelet matrix
    const codes1padded = codes.map((d, i) => ((d << (L - l[i])) >>> 0) + 2 ** (L - l[i]) - 1);
    const codes1paddedStrings = codes1padded.map((d, i) => d.toString(2).padStart(l[i], "0"));
    expect(codes1paddedStrings).toStrictEqual(
      ["111111", "11111", "1111", "01001", "00101", "00011", "01011", "00111", "000000", "000001"]
    );

    // For future reference, if we want to put this into a wavelet matrix,
    // we will need to know how many codes there are at each level.
    // The code below (from the original dev notebook) computes the histogram
    // of symbol counts per level.
    // It treats the `weights` as frequencies, ie. a weight of 20 indicates that 
    // some symbol from the original data occurs 20 times. Thereofre there are 55 symbols in this example.
    //
    const symbols = weights.flatMap((w, i) => Array.from({ length: w }, () => codes1padded[i]));
    // if we pass this into our WM constructor, it knows when to bail out at each level --
    // if the indices for the next level are >= the count at that level, then the code is already finished.
    const symCountsPerLevel = new Uint32Array(L); // histogram of counts
    for (let i = 0; i < l.length; i++) {
      symCountsPerLevel[l[i] - 1] += weights[i];
    }
    // iterate over [0, symCountsPerLevel.length-1) in reverse
    for (let i = symCountsPerLevel.length - 1; i-- > 0;) {
      symCountsPerLevel[i] += symCountsPerLevel[i + 1];
    }

    expect(symCountsPerLevel).toStrictEqual(new Uint32Array([55, 35, 18, 18, 12, 2]));
  });
});
