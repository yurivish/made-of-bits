import * as bits from './bits.js';

// todo: test error behavior for all assertions

export default {
  "BLOCK_BITS": {
    "is a power of two": [Number.isInteger(Math.log2(bits.BLOCK_BITS)), true],
  },
  "u32": {
    "handles signed integers": [bits.u32(-1), -1 >>> 0],
    // "truncates numbers beyond 2^32 - 1": [bits.u32(2 ** 32), 0], // todo: test that this throws an error now
  },
  "blockBitOffset": {
    "zero": [bits.blockBitOffset(0), 0],
    "BLOCK_BITS": [bits.blockBitOffset(bits.BLOCK_BITS), 0],
    "n*BLOCK_BITS + 7": [bits.blockBitOffset(10 * bits.BLOCK_BITS + 7), 7],
  },
  "blockIndex": {
    "zero": [bits.blockIndex(0), 0],
    "BLOCK_BITS": [bits.blockIndex(bits.BLOCK_BITS), 1],
    "n*BLOCK_BITS + 7": [bits.blockIndex(10 * bits.BLOCK_BITS + 7), 10],
  },
  "oneMask": {
    "handles zero": [bits.oneMask(0), 0],
    // note: testing 0..32 w /0b0*1*/ could be a use case for test code generation with <Gen>
    "handles 13": [bits.oneMask(13), 0b1111111111111],
    "handles 32": [bits.oneMask(32), bits.u32(2 ** 32 - 1)],
  },
};
