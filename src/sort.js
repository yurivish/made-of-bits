// Sort comparators for ascending & descending are sourced from d3-array:
// https://github.com/d3/d3-array/tree/main/src
/**
 * @param {number | null} a
 * @param {number | null} b
 */
export function ascending(a, b) {
  return a == null || b == null ? NaN
    : a < b ? -1 
      : a > b ? 1 
        : a >= b ? 0
          : NaN;
}

/**
 * @param {number | null} a
 * @param {number | null} b
 */
export function descending(a, b) {
  return a == null || b == null ? NaN
    : b < a ? -1
      : b > a ? 1
        : b >= a ? 0
          : NaN;
}