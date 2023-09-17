import { DEBUG, assert } from "./assert.js";

// Docs for JS bitwise operators:
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Expressions_and_Operators#bitwise_operators

// todo: document WHAT blocks this refers to (all of them):
// - bitbuf blocks
// - others?
// - should this be exported from bitbuf?
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
 * Based on an implementation by @ashaffer:
 * https://github.com/micro-js/popcount
 * @param {number} x
 */
export function popcount(x) {
  x -= (x >>> 1) & 0x55555555;
  x = (x & 0x33333333) + ((x >>> 2) & 0x33333333);
  x = (x + (x >>> 4)) & 0x0f0f0f0f;
  x += x >>> 8;
  x += x >>> 16;
  return x & 0x7f;
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
  var c = 32;
  x &= -x;
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
// E.g. select1(0b00001100, 0) == 3 and select1(0b00001100, 1) == 4
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