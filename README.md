# Testing

Use `assert` to assert invariants. In performance-critical code sections, guard uses of `assert` with `DEBUG &&` to enable the assertion to be compiled away in non-debug builds:

```
// incuding a message is optional, but encouraged.
assert(x <= 5, 'x cannot exceed 5');
DEBUG && assert(expensiveCheck(), 'expensive check must succeed');
```

## Some errors are thrown only in debug mode

Our tests run in debug mode and contain checks for errors that are thrown only in debug mode. For example, performance sensitive functions such as getting elements from a bit buffer will perform bounds checking in debug mode but not in release mode. 

In release mode, unexpected behavior may occur. This design decision may be revisited if such errors begin to come up in practice despite the debug mode errors.

# TypeScript

[] consider enabling the [noUncheckedIndexedAccess](https://www.typescriptlang.org/tsconfig#noUncheckedIndexedAccess) option.