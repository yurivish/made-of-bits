// @ts-ignore
// The `globalThis &&` allows us to substitute `globalThis` with `false`, and
// ESBuild's constant propagation will remove this if statement at build time
if (globalThis && globalThis.DEBUG === undefined) {
  Object.defineProperty(globalThis, 'DEBUG', { value: true });
}
