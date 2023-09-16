# Testing

## Some errors are thrown only in debug mode

Our tests run in debug mode and contain checks for errors that are thrown only in debug mode. For example, performance sensitive functions such as getting elements from a bit buffer will perform bounds checking in debug mode but not in release mode. 

In release mode, unexpected behavior may occur. This design decision may be revisited if such errors begin to come up in practice despite the debug mode errors.

# TypeScript

[] consider enabling the [noUncheckedIndexedAccess](https://www.typescriptlang.org/tsconfig#noUncheckedIndexedAccess) option.