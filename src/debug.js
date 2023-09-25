// @ts-ignore
// ESBuild's constant propagation will remove this if statement at build time
globalThis && !('DEBUG' in globalThis) &&
  Object.defineProperty(globalThis, 'DEBUG', { value: true });

