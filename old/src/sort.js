// These are like d3.ascending and d3.descending, but do not handle null, undefined, or NaN values.
// We check for invalid values and debug asserts so they should be safe to use in bit vector builders,
// Which saves us a dependency and improves performance vs. copying the d3 impls into this file.

/**
 * @param {number} a
 * @param {number} b
 */
export function ascending(a, b) { return a < b ? -1 : a > b ? 1 : 0; };

/**
 * @param {number} a
 * @param {number} b
 */
export function descending(a, b) { return a < b ? -1 : a > b ? 1 : 0; };