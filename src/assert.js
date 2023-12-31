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
 * 
 * @param {boolean} condition
 * @param {string | (() => string) } [message] - error message as a string or zero-argument function, 
 * to allow deferring the evaluation of an expensive message until the time an error occurs.
 */
export function assert(condition, message) {
  const prefix = 'assertion error';
  if (condition !== true) {
    const text = typeof message === "function" ? message() : message;
    throw new Error(text === undefined ? prefix : `${prefix}: ${text}`);
  }
};

/**
 * @param {number} x
 */
export function assertSafeInteger(x) {
  assert(Number.isSafeInteger(x), () => `expected safe integer, got ${x}`);
}

/**
 * @param {number} x
 */
export function assertNonNegative(x) {
  assert(x >= 0, () => `expected non-negative number, got ${x}`);
};

/**
 * @param {number} x
 */
export function assertInteger(x) {
  assert(Number.isInteger(x), () => `expected integer, got ${x}`);
};

/**
 * @param {any} x
 */
export function assertDefined(x) {
  assert(x !== undefined, 'expected a defined value, got undefined');
};


export const logNoLineNumbers = (/** @type {any[]} */ ...args) => {
  queueMicrotask(console.log.bind(console.log, ...args));
};

// export const log = console.log.bind(console.log);

export const log = logNoLineNumbers;