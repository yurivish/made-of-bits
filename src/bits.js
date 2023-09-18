import { DEBUG, assert } from "./assert.js";

// Docs for JS bitwise operators:
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Expressions_and_Operators#bitwise_operators

// todo: document WHAT blocks this refers to (all of them):
// - bitbuf blocks
// - others?
// - should this be exported from bitbuf?

// todo:
// - store a BLOCK_ARR = Uint32Array here as well? Then BLOCK_BITS = 8 * BLOCK_ARR.BYTES_PER_ELEMENT
// - a BLOCK_ARR of Uint8Array would make debugging easier...
// - we could make this a 'dynamic const', taking on parameters at run time somehow... maybe.
// Or we could build "templates' by generating a different version of eg. the BitBuf
// for each block size. (similar to zig templates)
export const BLOCK_BITS = 32; // todo: rename to BLOCK_SIZE? 
export const BLOCK_BITS_LOG2 = Math.log2(BLOCK_BITS);

/**
 * Coerces x to an unsigned 32-bit unsigned integer. This is provided as
 * a convenience function on top of unsigned shift that does some sanity
 * checks in debug mode.
 * @param {number} x
 */
export function u32(x) { 
  DEBUG && assert(Number.isInteger(x));
  DEBUG && assert(x >= 0 && x < 2 ** 32);
  return x >>> 0;
}

/**
 * Bit index of the `x`-th bit within its block (mask off the high bits)
 * @param {number} x
 */
export function blockBitOffset(x) {
  return x & (BLOCK_BITS - 1);
}

/**
 * Block index of the block containing the `x`-th bit
 * @param {number} x
 */
export function blockIndex(x) {
  return x >>> BLOCK_BITS_LOG2;
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

// TODO: test
/**
 * Returns the number of 1-bits in the binary representation of `x`.
 * Based on an implementation from Bit Twiddling Hacks:
 * https://graphics.stanford.edu/~seander/bithacks.html#CountBitsSetParallel
 * An explanation of the SWAR approach: 
 * https://stackoverflow.com/questions/109023/count-the-number-of-set-bits-in-a-32-bit-integer/109025#109025
 * @param {number} x
 */
export function popcount(x) {
  let v = (x | 0) - ((x >>> 1) & 0x55555555);
  v = (v & 0x33333333) + ((v >> 2) & 0x33333333);
  return (((v + (v >> 4)) & 0xf0f0f0f) * 0x1010101) >>> 24;
}
// TODO: test
/**
 * Returns the number of trailing 0-bits in the binary representation of `x`.
 * Like `Math.clz32` but for trailing rather than leading zeros.
 * Based on an implementation by @mikolalysenko:
 * https://github.com/mikolalysenko/count-trailing-zeros
 * @param {number} x
 */
export function trailing0(x) {
  x &= -x;
  let c = 32;
  if (x) c--;
  if (x & 0x0000ffff) c -= 16;
  if (x & 0x00ff00ff) c -= 8;
  if (x & 0x0f0f0f0f) c -= 4;
  if (x & 0x33333333) c -= 2;
  if (x & 0x55555555) c -= 1;
  return c;
}

// Return the position of the k-th least significant set bit.
// Assumes that x has at least k set bits.
// E.g. select1(0b1100, 0) === 2 and select1(0b1100, 1) === 3
//
// Will return 32 if the requested bit does not exist, eg. select1(0b1100, 2) === 32
//
// todo: is there a way to do doctests with node-tap or vite?
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
// pub fn select1<T: BitBlock>(mut x: T, k: u32) -> u32 {
//     // Unset the k-1 preceding 1-bits
//     for _ in 0..k {
//         x &= x - T::one();
//     }
//     x.trailing_zeros()
// }

/**
 * NOTE: Will return 32 if 
 * // todo: clarify that indices are 0-based and that it will return 32 if there is no kth 1-bit.
 * note that this is linear
 * @param {number} x
 * @param {number} k
 */
export function select1(x, k) {
  // Unset the k-1 preceding 1-bits
  for (let i = 0; i < k; i++) x &= x - 1;
  return trailing0(x);
}