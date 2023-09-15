import { DEBUG, assert } from "./assert.js";

// Docs for JS bitwise operators:
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Expressions_and_Operators#bitwise_operators

export const BLOCK_BITS = 32;
export const BLOCK_BITS_LOG2 = Math.log2(BLOCK_BITS);

// todo: I want to better understand when it's necessary to use an 
// unsigned right shift to turn a number into 32-bit number and
// when it's okay not to do that.
/**
 * Coerce x to an unsigned 32-bit integer.
 * @param {number} x
 */
export function u32(x) { 
  DEBUG && assert(Number.isInteger(x));
  DEBUG && assert(x < 2 ** 32);
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
 * @param {number} n
 */
export function oneMask(n) {
  DEBUG && assert(Number.isInteger(n));
  DEBUG && assert(n <= 32, `oneMask can only create masks for 32-bit integers`);
  if (n === 0) return 0 >>> 0;
  return 0xffffffff >>> (32 - n);
}