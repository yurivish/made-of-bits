import { assert, assertSafeInteger } from "./assert.js";

// Docs for JS bitwise operators:
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Expressions_and_Operators#bitwise_operators

// Used by the buffer types – IntBuf and BitBuf. We don't plan to change the block type at runtime,
// so we just define the constants here, and they're imported where needed (eg. the DenseBitVec).
// It's convenient to have the block size be malleable, especially for visualization purposes, where
// being able to show examples with 8-bit blocks is a nice option.
export const BasicBlockArray = Uint32Array;
export const BasicBlockSize = 8 * BasicBlockArray.BYTES_PER_ELEMENT;
export const BasicBlockSizePow2 = Math.log2(BasicBlockSize);

/**
 * Bit index of the `n`-th bit within its block (mask off the high bits)
 * @param {number} n
 */
export function basicBlockBitOffset(n) {
  return n & (BasicBlockSize - 1);
} 

/**
 * Block index of the block containing the `n`-th bit
 * @param {number} n
 */
export function basicBlockIndex(n) {
  return n >>> BasicBlockSizePow2;
}

/**
 * Returns the largest index for which `pred` returns true, plus one.
 * If the predicate does not return true for any index, returns 0.
 * The predicate function `pred` is required to be monotonic, ie. 
 * to return `true` for all inputs below some cutoff, and `false`
 * for all inputs above that cutoff.
 * 
 * This implementation is adapted from https://orlp.net/blog/bitwise-binary-search/
 * 
 * That post contains optimized versions of this function, but here I opted for the
 * clearest implementation, at a slight performance cost.
 * 
 * @param {number} n
 * @param {(index: number) => boolean} pred
 */
export function partitionPoint(n, pred) {
  DEBUG && assert(n < 2 ** 32);
  DEBUG && assertSafeInteger(n);
  let b = 0;
  let bit = bitFloor(n);
  while (bit !== 0) {
    const i = ((b | bit) - 1) >>> 0;
    if (i < n && pred(i)) {
      b |= bit;
    }
    bit >>>= 1;
  }
  return b >>> 0;
}

/**
 * If x is not zero, calculates the largest integral power of two that is not greater than x.
 * If x is zero, returns zero.
 * Like the function in the C++ standard library: https://en.cppreference.com/w/cpp/numeric/bit_floor
 * @param {number} n
 */
export function bitFloor(n) {
  DEBUG && assert(n < 2 ** 32);
  if (n === 0) {
    return 0;
  }
  const msb = 31 - Math.clz32(n);
  return (1 << msb) >>> 0;
}

/**
 * Coerces x to an unsigned 32-bit unsigned integer. This is provided as
 * a convenience function on top of unsigned shift that does some sanity
 * checks in debug mode.
 * @param {number} x
 */
export function u32(x) {
  DEBUG && assert(Number.isInteger(x));
  // Allow bit patterns representing negative numbers, eg. 1 << 31
  DEBUG && assert(Math.abs(x) < 2 ** 32);
  return x >>> 0;
}

/**
 * Returns an unsigned 32-bit integer with its bottom `n` bits set.
 * @param {number} n
 */
export function oneMask(n) {
  DEBUG && assert(Number.isInteger(n));
  DEBUG && assert(n >= 0 && n <= 32, 'oneMask can only create masks for 32-bit integers');
  if (n === 0) return 0 >>> 0;
  return 0xffffffff >>> (32 - n);
}

/**
 * Returns the number of 1-bits in the binary representation of `x`.
 * Based on an implementation from Bit Twiddling Hacks:
 * https://graphics.stanford.edu/~seander/bithacks.html#CountBitsSetParallel
 * An explanation of the SWAR approach: 
 * https://stackoverflow.com/questions/109023/count-the-number-of-set-bits-in-a-32-bit-integer/109025#109025
 * @param {number} n
 */
export function popcount(n) {
  DEBUG && assert(n < 2 ** 32);
  let v = (n | 0) - ((n >>> 1) & 0x55555555);
  v = (v & 0x33333333) + ((v >> 2) & 0x33333333);
  return (((v + (v >> 4)) & 0xf0f0f0f) * 0x1010101) >>> 24;
}
/**
 * Returns the number of trailing 0-bits in the binary representation of `n`.
 * Like `Math.clz32` but for trailing rather than leading zeros.
 * Based on an implementation by @mikolalysenko:
 * https://github.com/mikolalysenko/count-trailing-zeros
 * I wonder whether removing the branches would perform better:
 * eg. `c -= 16 * ((n & 0x0000ffff) !== 0); 
 * @param {number} n
 */
export function trailing0(n) {
  DEBUG && assert(n < 2 ** 32);
  n &= -n;
  let c = 32;
  if (n) c--;
  if (n & 0x0000ffff) c -= 16;
  if (n & 0x00ff00ff) c -= 8;
  if (n & 0x0f0f0f0f) c -= 4;
  if (n & 0x33333333) c -= 2;
  if (n & 0x55555555) c -= 1;
  return c;
}

// Return the position of the k-th least significant set bit.
// Assumes that x has at least k set bits.
// E.g. select1(0b1100, 0) === 2 and select1(0b1100, 1) === 3
//
// Will return 32 if the requested bit does not exist, eg. select1(0b1100, 2) === 32
//
// As an aside, if we're interested in potentially more efficient approaches,
// there is a broadword select1 implementation in the `succinct` package by
// Jesse A. Tov, provided under an MIT license: https://github.com/tov/succinct-rs
//
// An updated version of the paper is here: https://vigna.di.unimi.it/ftp/papers/Broadword.pdf
// If we use this, here are some items for future work:
// - Benchmark comparisons with the iterative select1 nelobelow
// - Use simd128 to accelerate u_le8, le8, and u_nz8
// - Implement 32-bit, 16-bit, and 8-bit select1
// - Write my own tests (the original file had tests, but I'd like to practice writing my own)

/**
 * Return the index of the `k`-th 1-bit of `n`, from the LSB upwards.
 * Returns 32 if there is no k-th 1-bit.
 * Note that the time complexity is linear in k, but constant since k <= 31.
 * https://lemire.me/blog/2018/02/21/iterating-over-set-bits-quickly/
 * @param {number} n
 * @param {number} k
 */
export function select1(n, k) {
  DEBUG && assert(n < 2 ** 32);
  // Unset the k-1 preceding 1-bits
  for (let i = 0; i < k; i++) n &= n - 1;
  return trailing0(n);
}


// Adapted from https://graphics.stanford.edu/~seander/bithacks.html#ReverseParallel
/**
 * @param {number} v
 */
export function reverseBits32(v) {
  DEBUG && assert(
    v <= 0xffffffff,
    "cannot reverse bits of an integer greater than 2^32-1"
  );
  // unsigned int v; // 32-bit word to reverse bit order
  // swap odd and even bits
  v = ((v >>> 1) & 0x55555555) | ((v & 0x55555555) << 1);
  // swap consecutive pairs
  v = ((v >>> 2) & 0x33333333) | ((v & 0x33333333) << 2);
  // swap nibbles ...
  v = ((v >>> 4) & 0x0f0f0f0f) | ((v & 0x0f0f0f0f) << 4);
  // swap bytes
  v = ((v >>> 8) & 0x00ff00ff) | ((v & 0x00ff00ff) << 8);
  // swap 2-byte long pairs
  v = (v >>> 16) | (v << 16);
  return v >>> 0;
}

/**
 * @param {number} v
 * @param {number} numBits
 */
export function reverseLowBits(v, numBits) {
  DEBUG && assert(numBits <= 32, "reverse more than 32 bits");
  return reverseBits32(v) >>> (32 - numBits);
}