// - DEBUG && assert (compiled away in release)
// - plain assert (runtime error)
// - utility asserts for type narrowing, eg. assert.isInteger or similar
// thoughts
// - We have two orthogonal concerns. One is debug versus non-debug assert, 
//   and the other is that we want high-level utilities to do the assertion.
//   For example, always asserting something is a safe integer. So we should have a nice interface that lets us do both of these.
// Solution: DEBUG &&: https://github.com/evanw/esbuild/issues/2063
// - can have DEBUG && assertInteger(x);

/**
 * @param {boolean} condition
 * @param {string} [message]
 */
export function assert(condition, message) {
  const prefix = 'assertion error';
  if (condition !== true) throw new Error(message === undefined ? prefix : `${prefix}: ${message}`);
};

/**
 * @param {number} x
 */
export function assertSafeInteger(x) {
  assert(Number.isSafeInteger(x), 'expected safe integer');
};

/**
 * @param {number} x
 */
export function assertInteger(x) {
  assert(Number.isInteger(x), 'expected integer');
};

/**
 * @param {any} x
 */
export function assertNotUndefined(x) {
  assert(x !== undefined, 'expected non-undefined');
};


export const logNoLineNumbers = (/** @type {any[]} */ ...args) => {
  queueMicrotask(console.log.bind(console.log, ...args));
};

// export const log = console.log.bind(console.log);

export const log = logNoLineNumbers;