// Debug mode is set to true by default using the definition below.
// When bundling with ES build, we have the option of setting both 
// `globalThis` and `DEBUG` global constants to false, which will
// elide the debug calls.

// @ts-ignore
// ESBuild's constant propagation will remove this if statement at build time
// if `DEBUG` is set to false.
globalThis && !('DEBUG' in globalThis) &&
  Object.defineProperty(globalThis, 'DEBUG', { value: true });

