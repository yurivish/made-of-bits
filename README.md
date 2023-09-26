# To do
[] impl a "padded bitvec" with 0 or 1 padding and an internal offset bitvec obeying the interface.
[] run tests on git commit before allowing a commit to go in? maybe too strict; maybe we should disallow merges to release or something unless tsc / testing both pass.
[] figure out how to use coverage info (maybe try switching to node-tap before we do this)
[] figure out if we can add docstrings to the interface and not need them on the individual types
[] fix clicking to navigate to lines in terminus: https://forum.sublimetext.com/t/clicking-on-build-errors-doesnt-work-other-stuff/50728/4
[] https://www.totaltypescript.com/tsconfig-cheat-sheet
[] dense bitvec: use the s0 index if there is no s1 index? at least add a note/todo for it in the code.
[] work on quadvector (r00, r01, r11; and it is nice that there is a cap on the total number of select samples) and a quad wavelet matrix on top (supporting a limited set of ops).
   - this will potentially have a 1-bit layer on the top
[] look through package.json and understand the meaning & purpose of every field. Some were just copied from https://vitejs.dev/guide/build.html#library-mode...
[] For performance, investigate using bid operations to cast intermediate results to unsigned integers because this might enable jit optimizations.
[] play around with an `Opaque<x>` style nominal type thing but just using type coersion: `x as SelectSample` where `SelectSample` has no methods, and is not also a number. Then cast back when we know, inside the function that accepts `SelectSample`s.
[] remove types.d.js?

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