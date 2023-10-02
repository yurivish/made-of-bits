import { assert } from './assert.js';

/**
 * @param {any[]} arr - array to track
 * @param {object[]} log - array to append access log messages to
 * @param {string} tag - string tag for each log message
 */
export function trackedArray(arr, log, tag) {
  const handler = {
    /**
     * @param {any[]} target
     * @param {any} prop
     * @param {any} receiver
     * @returns {any}
     */
    get(target, prop, receiver) {
      assert(target === arr);
      // Track array accesses by recording integer indexing operations.
      if (typeof prop === "string") {
        const index = Number(prop);
        if (Number.isInteger(index)) {
          log.push({ tag, index, type: "get", });
        }
      }
      // Change the receiver from the proxy to the target, which fixes errors
      // with typed arrays (they internally perform receiver checks), though
      // it does mean that the proxy will not monitor internal self-calls.
      return Reflect.get(target, prop, receiver === proxy ? target : receiver);
    }
  };
  // @ts-ignore
  const proxy = new Proxy(arr, handler);
  return proxy;
}