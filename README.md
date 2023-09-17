# To do
[] look through package.json and understand the meaning & purpose of every field. Some were just copied from https://vitejs.dev/guide/build.html#library-mode...
[] For performance, investigate using bid operations to cast intermediate results to unsigned integers because this might enable jit optimizations.
[] play around with an `Opaque<x>` style nominal type thing but just using type coersion: `x as SelectSample` where `SelectSample` has no methods, and is not also a number. Then cast back when we know, inside the function that accepts `SelectSample`s.

# Documentation
[] Change all comments that say "returns ..." to use @returns doc syntax

# Creating a Live Development Experience
## Removing friction'

[] figure out how to jump to my local files, w/ sourcemaps etc.: https://developer.chrome.com/docs/devtools/workspaces/?utm_source=devtools
  - answer so far: enable inline sourcemaps & drag the folder into the workspace in the Sources tab in Chrome
[x] ctrl-click to jump to definition in sublime. super useful!
[x] lsp-typescript
[] lots of strict modes
[] figure out how to jump from an error in the JS console in Chrome all the way to the file in my code editor. [google search to start](https://www.google.com/search?q=chrome+dev+tools+open+local+code+source+)
[x] code on left, tests on right, auto re-running on save and showing the relevant test file. (thanks, vitest)
[] try using a terminal inside subline so I can do the above with 2 tabs (trying [Terminus](https://github.com/randy3k/Terminus))
[] try copilot
[] see if I can also pin the output of tests while doing my development with esbuild
[] see if i can prevent terminus panel from closing with esc when it is open
   - related: https://github.com/randy3k/Terminus/issues/58


# Testing

[] test bits.popcount
[] test bits.trailing0
[] test `DenseBitVec`

[] Investigate whether there is a way to force parentheses for ambiguous arithmetic expressions.

[] We should consider switching to [node-tap](https://node-tap.org/) once version 18 is out because it appears to be a more mature tool.

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