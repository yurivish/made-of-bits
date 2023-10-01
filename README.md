# To do
[] rename this to notes
[] make a plain README.md noting this is wip
[] do a pass through the files to ensure it is all ready to be made public (eg. remove gen.js and other unused code?)
[] decide whether to rename 'multiplicity' to 'repeats', eg. `hasRepeats`

# Visualizations

- https://observablehq.com/@yurivish/bitvectors-with-runs-3
- explore amit's page here: https://www.redblobgames.com/making-of/little-things/

# Notes

- Allow greater than two to the 32 bits in sparse vector by controlling the split point 
- Approximate number of bites because that’s what’s gonna always fit in the 32 bit integer 
- We can do the dynamic regeneration trick for avoiding megamorphic default functions like in that blog post. I read today about optimizing versus rust. 
- We have a encoded vector builder that lets you put in runs, but also supports, adding runs of length one to be of the same interface as before,  allowing us to reuse tests 
- Try always less-than comparators: https://matklad.github.io/2023/09/13/comparative-analysis.html
- "Tour of bit vecs" post.
   - Table of properties: multiplicity: no, low (explicitly stored repetitions), yes (eg occupancy plus count)
- Is there another way to look at the run length bit vector as if it’s encoding account for not only each one bit but also each zero bit? Maybe we can use this to more elegantly express the wavelet matrix weighted quintile thing
   - Basically the aligned rank0 and rank1 except you just supply the index - these become the normal rank ops. Visually it’s like instead of stacking the ones and zeros horizontally we’re stacking them vertically.
   - Maybe there’s another structure for this case where you want multiplicity of both ones and zeros. Store and occupancy, bit vector and store two sparse count vectors with the cumulative number of zeros and cumulative number of ones, each corresponding to the respective entry in the occupancy vector. The case, with only counts for one becomes a special case, where you admit, the zero vector, and assume a count of one for each zero. Maybe we store one minus the count, though this might get weird with select.
   - Or maybe it’s more of a plus one minus one kind of bit vec. by default the zeros all have count zero so it acts like a regular bit vector. But if you give them a count, then they become negative weight.
- Can we do all intercepting with proxy from the outside? Intercept get index and append to log.
- add a ZeroPadded: BitVec wrapper for padding on both sides
- If we have Huffman encoding, then we can store trees in a WM with each symbol only taking as long as its depth or frequency or something, rather than each symbol having the same number of bits as the maximum depth of the tree.
- Idea: work on package docs next

[] We should test all zeros vector, and all ones bit vector   
[] try imlementing a simple fm index: https://drops.dagstuhl.de/opus/volltexte/2023/18357/


# MultiSet
[] consider calling it 'repetition' rather than 'multiplicity'?
[] call it set or vec?
[] how to allow/disallow rank0?
[] should we distinguish multisets at the type level? feels weird

# Design

[] Try a variable width font for the zoomable bit vector :)
[] Make a bit vec doc tech note. Audience: me
[] Try scrivener. Import transcripts and audio notes

# To do
[] Hist constructor: accept {m,r,n} or {a,b,n} or even any two of a b c with m and r as aliases
[] consider allowing fixed with integer backing buffers for the sorted array vector
[] impl a "padded bitvec" with 0 or 1 padding and an internal offset bitvec obeying the interface.
[] run tests on git commit before allowing a commit to go in? maybe too strict; maybe we should disallow merges to release or something unless tsc / testing both pass.
[] figure out how to use coverage info (maybe try switching to node-tap before we do this)
[] figure out if we can add docstrings to the interface and not need them on the individual types
[] fix clicking to navigate to lines in terminus: https://forum.sublimetext.com/t/clicking-on-build-errors-doesnt-work-other-stuff/50728/4
[] https://www.totaltypescript.com/tsconfig-cheat-sheet
[x] dense bitvec: use the s0 index if there is no s1 index? at least add a note/todo for it in the code. [note added]
[] work on quadvector (r00, r01, r11; and it is nice that there is a cap on the total number of select samples) and a quad wavelet matrix on top (supporting a limited set of ops).
   - this will potentially have a 1-bit layer on the top
[] look through package.json and understand the meaning & purpose of every field. Some were just copied from https://vitejs.dev/guide/build.html#library-mode...
[] For performance, investigate using bit operations to cast intermediate results to unsigned integers because this might enable jit optimizations.
[x] play around with an `Opaque<x>` style nominal type thing but just using type coersion: `x as SelectSample` where `SelectSample` has no methods, and is not also a number. Then cast back when we know, inside the function that accepts `SelectSample`s.

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
[x] try using a terminal inside subline so I can do the above with 2 tabs (trying [Terminus](https://github.com/randy3k/Terminus))
[] try copilot
[] see if I can also pin the output of tests while doing my development with esbuild
[] see if i can prevent terminus panel from closing with esc when it is open
   - related: https://github.com/randy3k/Terminus/issues/58

# Wavelet matrix
[] Wm: one loop that goes up to 32 with bit operations, and then another one that does the rest with math operations up to 53

# Testing

[] test with a vastly larger universeSize than the number of bits
[] 
[x] test bits.popcount
[x] test bits.trailing0
[x] test `DenseBitVec`

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