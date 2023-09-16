import { DEBUG, assert } from "./assert.js";

// Docs for JS bitwise operators:
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Expressions_and_Operators#bitwise_operators

export const BLOCK_BITS = 32;
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