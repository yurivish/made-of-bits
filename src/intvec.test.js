import { IntVec } from "./intvec.js";

// todo:
// - check that the various error conditions behave correctly (pushing value < 0, pushing beyond end, ...)

// todo: assert this errors
// new IntVec(0, 0).get(0) 

/** @type {Record<string, any>} */
const tests = {};

{
  /** @type {Record<string, any>} */
  const test = {};

  const ints = new IntVec(4, 0);
  const values = [0, 0, 0, 0];

  // todo: assert pushing values > 0 errors

  for (let i = 0; i < values.length; i++) {
    const value = values[i];
    ints.push(value);
    test[`ints[${i}] = ${value}`] = [value, ints.get(i)];
  }

  tests['zero-bit IntVec'] = test;
}


{
  /** @type {Record<string, any>} */
  const test = {};

  const ints = new IntVec(4, 1);
  const values = [1, 0, 1, 0];

  // todo: assert pushing values > 0 errors

  for (let i = 0; i < values.length; i++) {
    const value = values[i];
    ints.push(value);
    test[`ints[${i}] = ${value}`] = [value, ints.get(i)];
  }

  tests['one-bit IntVec'] = test;
}


{
  /** @type {Record<string, any>} */
  const test = {};

  const ints = new IntVec(4, 5);
  const values = [10, 0, 31, 20];

  for (let i = 0; i < values.length; i++) {
    const value = values[i];
    ints.push(value);
    test[`ints[${i}] = ${value}`] = [value, ints.get(i)];
  }

  tests['5-bit IntVec'] = test;
}


{
  /** @type {Record<string, any>} */
  const test = {};

  const ints = new IntVec(5, 32);
  const values = [10, 0, 31, 20, 2 ** 32 - 1];

  for (let i = 0; i < values.length; i++) {
    const value = values[i];
    ints.push(value);
    test[`ints[${i}] = ${value}`] = [value, ints.get(i)];
  }

  tests['32-bit IntVec'] = test;
}


// todo: assert this errors
// ints.push(-1); // too small

// todo: assert this errors
// ints.push(40);  // too large



// todo: test error behavior for all assertions

export default tests;