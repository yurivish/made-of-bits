import { symbol } from 'd3';
import { assert } from './assert.js';
import { BitBuf } from './bitbuf.js';
import { oneMask, reverseLowBits, u32 } from './bits.js';
import { DenseBitVec } from './densebitvec.js';
import { bits } from './index.js';

// Implements a wavelet matrix, which is an efficient data structure for
// wavelet tree operations on top of a levelwise bitvector representation
//
// Nice description of wavelet trees:
//   https://www.alexbowe.com/wavelet-trees/
// Overview and uses of the wavelet tree:
//   https://www.sciencedirect.com/science/article/pii/S1570866713000610
// Original wavelet matrix paper:
//   https://users.dcc.uchile.cl/~gnavarro/ps/spire12.4.pdf
// Paper: Practical Wavelet Tree Construction:
//   https://dl.acm.org/doi/fullHtml/10.1145/3457197/
// Paper: New algorithms on wavelet trees and applications to information retrieval:
//   https://www.sciencedirect.com/science/article/pii/S0304397511009625/pdf?md5=32fe86d035e8a0859fd3a4b045e8b36b&pid=1-s2.0-S0304397511009625-main.pdf

// todo:
// - debug asserts (eg. validate ranges)
// - explain and test behavior of selectUpwards. I tried writing tests but couldn't actually figure out what it's supposed to do.
// - impl batch count function for density plots, using masks.

const rank1SamplesPow2 = 10;
const selectSamplesPow2 = 10;

export class WaveletMatrix {

  /**
   * @param {number[]} data
   * @param {number} [maxSymbol]
   */
  constructor(data, maxSymbol) {
    if (maxSymbol === undefined) {
      maxSymbol = data.reduce((a, b) => Math.max(a, b), 0);
    }
    assert(maxSymbol < 2 ** 32);
    const alphabetSize = maxSymbol + 1;
    const numLevels = Math.max(1, Math.ceil(Math.log2(alphabetSize)));
    // We implement two different wavelet matrix construction algorithms. One of them is more
    // efficient, but that algorithm does not scale well to large alphabets and also cannot
    // cannot handle element multiplicity because it constructs the bitvectors out-of-order.
    // It also requires O(2^num_levels) space. So, we check whether the number of data points
    // is less than 2^num_levels, and if so use the scalable algorithm, and otherise use the
    // the efficient algorithm.
    let /** @type {BitVec[]} */ bitVecs;
    if (data.length === 0) {
      // Create an empty bitvec since numLevels is 1
      bitVecs = [new DenseBitVec(new BitBuf(0), rank1SamplesPow2, selectSamplesPow2)];
    } else if (numLevels <= Math.floor(Math.log2(data.length))) {
      bitVecs = buildBitVecsSmallAlphabet(data, numLevels);
    } else {
      bitVecs = buildBitVecsLargeAlphabet(data, numLevels);
    };

    assert(bitVecs.length > 0);
    this.maxSymbol = maxSymbol;
    this.numLevels = bitVecs.length;
    this.maxLevel = this.numLevels - 1;
    this.length = bitVecs[0].universeSize;
    this.levels = bitVecs.map((bv, index) => ({
      nz: bv.numZeros,
      bit: u32(1 << (this.maxLevel - index)),
      bv
    }));
    this.defaultLevelMasks = bitVecs.map(() => bits.oneMask(32));
  }

  /**
   * @param {number} symbol
   * @param {Object} [options]
   * @param {{ start: number; end: number; }} [options.range]
   * @param {number} [options.ignoreBits]
   */
  locate(symbol, { range = Range(0, this.length), ignoreBits = 0 } = {}) {
    let precedingCount = 0;
    const numLevels = this.numLevels - ignoreBits;
    for (let i = 0; i < numLevels; i++) {
      const level = this.levels[i];
      const start = ranks(level, range.start);
      const end = ranks(level, range.end);
      // Check if the symbol's level bit is set to determine whether it should be mapped
      // to the left or right child node
      if ((symbol & level.bit) === 0) {
        // Go left
        range = Range(start.zeros, end.zeros);
      } else {
        // Count the symbols in the left child before going right
        precedingCount += end.zeros - start.zeros;
        range = Range(level.nz + start.ones, level.nz + end.ones);
      }
    }
    // precedingCount is the number of symbols less than `symbol`, restricted to the query range
    // range is the range of the symbol on the virtual bottom-most level, accounting for `ignoreBits`
    return { precedingCount, range };
  }

  /**
   * Number of symbols less than this one, restricted to the query range
   * @param {number} symbol
   * @param {Object} [options]
   * @param {{ start: number; end: number; }} [options.range]
   */
  precedingCount(symbol, { range = Range(0, this.length) } = {}) {
    return this.locate(symbol, { range }).precedingCount;
  }

  /**
   * Number of times the symbol appears in the query range.
   * We could also provide a more efficient rank operation that takes
   * an index and only does one rank per level.
   * @param {number} symbol
   * @param {Object} [options]
   * @param {{ start: number; end: number; }} [options.range]
   */
  count(symbol, { range = Range(0, this.length) } = {}) {
    const loc = this.locate(symbol, { range });
    return loc.range.end - loc.range.start;
  }

  /**
   * @param {number} k
   * @param {Object} [options]
   * @param {{ start: number; end: number; }} [options.range]
   */
  quantile(k, { range = Range(0, this.length) } = {}) {
    assert(0 <= k && k < this.length);
    let symbol = 0;
    for (const level of this.levels) {
      let start = ranks(level, range.start);
      let end = ranks(level, range.end);
      let leftCount = end.zeros - start.zeros;
      if (k < leftCount) {
        // Go left
        range = Range(start.zeros, end.zeros);
      } else {
        k -= leftCount;
        symbol += level.bit;
        range = Range(level.nz + start.ones, level.nz + end.ones);
      }
    }
    let count = range.end - range.start;
    return { symbol, count };
  }

  /**
   * This function abstracts the common second half of the select algorithm, once you've
   * identified an index on the "bottom" level and want to bubble it back up to translate
   * the "sorted" index from the bottom level to the index of that element in sequence order.
   * This function allows eg. external users of `locate` to efficiently select their chosen element.
   * 
   * Note that this function returns absolute indices. So all functions that rely on it also
   * return absolute indices, even when the user passes in a range.
   * 
   * @param {number} index
   * @param {Object} [options]
   * @param {number} [options.ignoreBits]
   */
  selectUpwards(index, { ignoreBits = 0 } = {}) {
    for (let i = this.numLevels - ignoreBits; i-- > 0;) {
      const level = this.levels[i];
      // `index` represents an index on the level below this one, which may be
      // the bottom-most 'virtual' layer that contains all symbols in sorted order.
      //
      // We want to determine the position of the element represented by `index` on
      // this level, which we can do by "mapping" the index up to its parent node.
      //
      // `level.nz` tells us how many bits on the level below come from left children of
      // the wavelet tree (represented by this wavelet matrix). If the index < nz, that
      // means that the index on the level below came from a left child on this level,
      // which means that it must be represented by a 0-bit on this level; specifically,
      // the `index`-th 0-bit, since the WT always represents a stable sort of its elements.
      //
      // On the other hand, if `index` came from a right child on this level, then it
      // is represented by a 1-bit on this level; specifically, the `index - nz`-th 1-bit.
      //
      // In either case, we can use bitvector select to compute the index on this level.
      if (index < level.nz) {
        // `index` represents a left child on this level, represented by the `index`-th 0-bit.
        index = level.bv.select0(index);
      } else {
        // `index` represents a right child on this level, represented by the `index-nz`-th 1-bit.
        index = level.bv.select1(index - level.nz);
      }
    }
    return index;
  }

  /**
   * Return the index of the k-th occurrence of the symbol in this wavelet matrix.
   * Note that this returns an absolute index, even if a range is specified.
   * 
   * @param {number} symbol
   * @param {Object} [options]
   * @param {number} [options.k]
   * @param {{ start: number; end: number; }} [options.range]
   * @param {number} [options.ignoreBits]
   */
  select(symbol, { k = 0, range = Range(0, this.length), ignoreBits = 0 } = {}) {
    if (symbol > this.maxSymbol) { 
      return null;
    }

    // Track the symbol down to a range on the bottom-most level we're interested in
    let loc = this.locate(symbol, { range, ignoreBits });
    let count = loc.range.end - loc.range.start;

    // If there are fewer than `k+1` copies of `symbol` in the range, return early.
    // `k` is zero-indexed, so our check includes equality.
    if (count <= k) {
      return null;
    }

    // Track the k-th occurrence of the symbol up from the bottom-most virtual level
    // or higher, if ignore_bits is non-zero.
    let index = loc.range.start + k;
    return this.selectUpwards(index, { ignoreBits });
  }

  /**
   * Same as select, but select the k-th instance from the back of the range.
   * 
   * @param {number} symbol
   * @param {Object} [options]
   * @param {number} [options.k]
   * @param {{ start: number; end: number; }} [options.range]
   * @param {number} [options.ignoreBits]
   */
  selectFromEnd(symbol, { k = 0, range = Range(0, this.length), ignoreBits = 0 } = {}) {
    if (symbol > this.maxSymbol) { 
      return null;
    }

    // Track the symbol down to a range on the bottom-most level we're interested in
    let loc = this.locate(symbol, { range, ignoreBits });
    let count = loc.range.end - loc.range.start;

    // If there are fewer than `k+1` copies of `symbol` in the range, return early.
    // `k` is zero-indexed, so our check includes equality.
    if (count <= k) {
      return null;
    }

    // Track the k-th occurrence of the symbol up from the bottom-most virtual level
    // or higher, if ignore_bits is non-zero.
    // The `- 1` is because the end of the range is exclusive
    let index = loc.range.end - k - 1;
    return this.selectUpwards(index, { ignoreBits });
  }

  /**
   * Returns the index of the first symbol less than `symbol` in the index range `range`.
   * ("First" here is based on sequence order; we will return the leftmost such index).
   * Implements the following logic:
   * selectFirstLessThanOrEqual = (arr, symbol, lo, hi) => {
   *   let i = arr.slice(lo, hi).findIndex((x) => x <= symbol);
   *   return i === -1 ? null : lo + i;
   * }
   * @param {number} symbol
   * @param {Object} [options]
   * @param {{ start: number; end: number; }} [options.range]
   */
  selectFirstLessThanOrEqual(symbol, { range = Range(0, this.length) } = {}) {
    let leftmostSymbol = 0; // leftmost symbol in the currently-considered wavelet tree node
    let best = bits.oneMask(32);
    let found = false;

    // The target range is a conceptual `Range(0, symbol + 1)`;
    // since the left extent of the target is always zero, we could optimize the containment checks.
    let targetEnd = symbol + 1;

    // The idea is to return the minimum select position across all the nodes that could
    // potentially contain the first symbol <= symbol.
    //
    // We find the first left node that is fully contained in the [0, symbol] symbol range,
    // and then we recurse into the right child if it is partly contained, and repeat.
    //
    // todo:
    // implement select[First|Last][Less|Greater]ThanOrEqual, Or perhaps a version that gets
    // the k-th element from the start/end of the range.

    for (let i = 0; i < this.numLevels; i++) {
      if (rangeIsEmpty(range)) {
        break;
      }
      // ignore all levels below this one when selecting
      const ignoreBits = this.numLevels - i;

      const level = this.levels[i];
      // value split points of left/right children
      const { left, mid, right } = split(level, leftmostSymbol);
      // range of left/right children
      // if this wavelet tree node is fully contained in the target range, update best and return.
      
      if (right <= targetEnd) {
        // The if condition above is conceptually equivalent to `rangeFullyContains(targetRange, Range(left, right)))`
        // it's a simplified version of that since the target range always starts at zero.
        const candidate = this.selectUpwards(range.start, { ignoreBits });
        return Math.min(best, candidate);
      }

      const start = ranks(level, range.start);
      const end = ranks(level, range.end);

      // otherwise, we know that there are two possibilities:
      // 1. the left node is partly contained and the right node does not overlap the target
      // 2. the left node is fully contained and the right node may overlap the target
      if (targetEnd < mid) {
        // The `if` condition above is conceptually equivalent to `!rangeFullyContains(targetRange, Range(left, mid))`
        // it's a simplified version of that since the target range always starts at zero.

        // we're in case 1, so refine our search range by going left
        range = Range(start.zeros, end.zeros);
      } else {
        if (start.zeros !== end.zeros) {
          // since this select is happening on the child level, un-ignore that level.
          const candidate = this.selectUpwards(start.zeros, { ignoreBits: ignoreBits - 1 });
          best = Math.min(best, candidate);
          found = true;
        }
        // go right
        leftmostSymbol += level.bit;
        range = Range(level.nz + start.ones, level.nz + end.ones);
      }
    }

    return found ? best : null;
  }

  /**
   * Return the majority element as `{ symbol, count }` if it exists, or `null` if it doesn't.
   * The majority element is one whose frequency (count) is larger than 50% of the range.
   * @param {{ end: any; start: any; }} range
   */
  simpleMajority(range) {
    const length = range.end - range.start;
    const halfLength = length >>> 1;
    const result = this.quantile(halfLength, { range });
    if (result.count > halfLength) {
      return result;
    } else {
      return null;
    }
  }

  // todo: fn k_majority(&self, k, range) { ... }
  // Returns the 1/k-majority. Ie. for k = 4, return the elements (if any) with
  // frequency larger than 1/4th (25%) of the specified index range.
  //   - note: could we use this with ignore_bits to check if eg. half of the values are in the bottom half/quarter?
  //   - ie. doing majority queries on the high bits lets us make some statements about the density of values across
  //     *ranges*. so rather than saying "these symbols have frequency >25%" we can say "these symbol ranges have
  //     frequency >25%", for power of two frequencies (or actually arbitrary ones, based on the quantiles...right?)
  // note: even more useful would be a k_majority_candidates function that returns all the samples, which can then be filtered down.

  /**
   * @param {number} index
   */
  get(index) {
    let symbol = 0;
    for (const level of this.levels) {
      if (level.bv.get(index) === 0) {
        // Go left
        index = level.bv.rank0(index);
      } else {
        symbol += level.bit;
        index = level.nz + level.bv.rank1(index);
      }
    }
    return symbol;
  }

  // todo: rank cache or similar
  // todo: consider using extent for symbol?
  // todo: consider using MaskExtent to avoid the extra sub/add instructions
  // todo: consider using an array with 3 consecutive u32 elements per (symbol, start, end)
  counts({ range = Range(0, this.length), symbolRange = Range(0, this.maxSymbol + 1), masks = this.defaultLevelMasks } = {}) {
    let xs = [{
      symbol: 0, // the leftmost symbol in the current node
      start: range.start, // index  range start
      end: range.end // index range end
    }];
    let nextLeft = xs.slice(0, 0);  // create these empty arrays via slicing 
    const nextRight = xs.slice(0, 0); // for type inference purposes

    for (let i = 0; i < masks.length; i++) {
      const mask = masks[i];
      const level = this.levels[i];
      const levelSymbolRange = MaskedRange(symbolRange.start, symbolRange.end, mask);

      // cache `ranks` results when contiguous ranges share an endpoint
      let xEnd = 0; // cache key
      let rankCache = ranks(level, xEnd); // cached value

      for (const x of xs) {
        // use the cache if the cache key matches
        const start = x.start == xEnd ? rankCache : ranks(level, x.start);
        const end = ranks(level, x.end);

        // update the cache
        xEnd = x.end;
        rankCache = end; 

        const symbol = x.symbol;
        const { left, right } = childSymbolRanges(level, symbol, mask);

        // if there are any left children, go left
        if (start.zeros !== end.zeros && rangesOverlap(levelSymbolRange, left)) {
          nextLeft.push({
            symbol, 
            start: start.zeros, 
            end: end.zeros
          }); 
        }

        // if there are any right children, set the level bit and go right
        if (start.ones !== end.ones && rangesOverlap(levelSymbolRange, right)) {
          nextRight.push({
            symbol: symbol + level.bit, 
            start: level.nz + start.ones, 
            end: level.nz + end.ones
          });
        }
      }

      // swap xs and nextLeft
      let tmp = xs;
      xs = nextLeft;
      nextLeft = tmp;

      // append the right to the left
      for (let i = 0; i < nextRight.length; i++) {
        xs.push(nextRight[i]);
      }

      // clear both for the next iteration
      nextLeft.length = nextRight.length = 0;
    }
    return xs;
  }
}

/**
 * @param {{ start: number; end: number; }} range
 */
function rangeIsEmpty(range) {
  return !(range.start < range.end);
}

// todo: document
/**
 * @param {{ nz: number; bit: number; bv: BitVec; }} level
 * @param {number} index
 */
function ranks(level, index) {
  let numOnes = level.bv.rank1(index);
  let numZeros = index - numOnes;
  return { zeros: numZeros, ones: numOnes };
}

// todo: document
/**
 * @param {number} start
 * @param {number} end
 */
function Range(start, end) {
  return { start, end };
}

// todo: document
/**
 * @param {{ start: number; end: number; }} a
 * @param {{ end: number; start: number; }} b
 */
function rangesOverlap(a, b) {
  return a.start < b.end && b.start < a.end;
}

/**
 * Return true if range `a` fully contains range `b`
 * @param {{ start: number; end: number; }} a
 * @param {{ start: number; end: number; }} b
 */
function rangeFullyContains(a, b) {
  // if a starts before b, and a ends after b.
  return a.start <= b.start && a.end >= b.end;
}

// todo: document
/**
 * @param {number} start
 * @param {number} end
 * @param {number} mask
 */
function MaskedRange(start, end, mask) {
  // The end is exclusive so we need to make sure to mask the true endpoint.
  return { start: start & mask, end: ((end - 1) & mask) + 1 };
}

// todo: document
/**
 * @param {{ nz: number; bit: number; bv: BitVec; }} level
 * @param {number} leftSymbol
 */
function split(level, leftSymbol) {
  return {
    left: leftSymbol, 
    mid: leftSymbol + level.bit,
    right: leftSymbol + level.bit + level.bit 
  };
}

// todo: document
/**
 * @param {{ nz: number; bit: number; bv: BitVec; }} level
 * @param {number} leftSymbol
 * @param {number} mask
 */
function childSymbolRanges(level, leftSymbol, mask) {
  const { left, mid, right } = split(level, leftSymbol);
  return {
    left: MaskedRange(left, mid, mask),
    right: MaskedRange(mid, right, mask),
  };
}

/**
 * Wavelet matrix construction algorithm that takes space proportional to the alphabet size (which is 2^numLevels).
 * From the paper "Practical Wavelet Tree Construction" (see link in comment at the top of this file)
 * @param {number[]} data
 * @param {number} numLevels
 */
function buildBitVecsSmallAlphabet(data, numLevels) {
  assert(numLevels > 0);
  const levels = Array.from({ length: numLevels }, () => new BitBuf(data.length));
  const hist = new Uint32Array(u32(1 << numLevels));
  const borders = new Uint32Array(u32(1 << numLevels));
  const maxLevel = numLevels - 1;

  {
    // Count symbol occurrences and fill the first bitvector, whose bits
    // can be read from MSBs of the data in its original order.
    const level = levels[0];
    const levelBit = u32(1 << maxLevel);
    for (let i = 0; i < data.length; i++) {
      const d = data[i];
      hist[d] += 1;
      if ((d & levelBit) > 0) {
        level.setOne(i);
      }
    }
  }

  // Construct the other levels bottom-up
  for (let l = numLevels - 1; l > 0; l--) {
    // The number of wavelet tree nodes at this level
    const numNodes = u32(1 << l);

    // Compute the histogram based on the previous level's histogram
    for (let i = 0; i < numNodes; i++) {
      // Update the histogram in-place
      hist[i] = hist[2 * i] + hist[2 * i + 1];
    }

    // Get starting positions of intervals from the new histogram
    borders[0] = 0;
    for (let i = 1; i < numNodes; i++) {
      // Update the positions in-place. The bit reversals map from wavelet tree
      // node order to wavelet matrix node order, with all left children preceding
      // the right children.
      let prevIndex = reverseLowBits(i - 1, l);
      borders[reverseLowBits(i, l)] = borders[prevIndex] + hist[prevIndex];
    }

    // Fill the bit vector of the current level
    const level = levels[l];
    const levelBitIndex = maxLevel - l;
    const levelBit = u32(1 << levelBitIndex);

    // This mask contains all ones except for the lowest levelBitIndex bits.
    // This is a bit subtle since the negation operates only on the 32-bit value,
    // but this works so long as we never build elements with value greater than 2^32
    const bitPrefixMask = ~oneMask(levelBitIndex);
    for (const d of data) {
      // Get and update position for bit by computing its bit prefix from the
      // MSB downwards which encodes the path from the root to the node at
      // this level that contains this bit
      let node_index = (d & bitPrefixMask) >>> (levelBitIndex + 1);
      const p = borders[node_index];
      // Set the bit in the bitvector
      if ((d & levelBit) > 0) {
        level.setOne(p);
      }
      borders[node_index]++;
    }
  }

  // todo: configurable dense bitvec parameters
  return levels.map(d => new DenseBitVec(d, rank1SamplesPow2, selectSamplesPow2));
}

/**
 * Wavelet matrix construction algorithm that takes space proportional to data.length rather
 * than the alphabet size, allowing for sparse alphabets up to 2^32, eg. a symbol space of [0, 2^32).
 * From the paper "Practical Wavelet Tree Construction" (see link in comment at the top of this file)
 * @param {number[]} data
 * @param {number} numLevels
 */
function buildBitVecsLargeAlphabet(data, numLevels) {
  assert(numLevels > 0);
  const levels = [];
  const maxLevel = numLevels - 1;

  // For each level, stably sort the datapoints by their bit value at that level.
  // Elements with a zero bit get sorted left, and elements with a one bits
  // get sorted right, which is effectvely a bucket sort with two buckets.
  const right = [];

  for (let l = 0; l < maxLevel; l++) {
    const levelBit = u32(1 << (maxLevel - l));
    const bits = new BitBuf(data.length);
    // Stably sort all elements with a zero bit at this level to the left, storing
    // the positions of all one bits at this level in `bits`.
    // We retain the elements that went left, then append those that went right.
    let n = 0;
    for (let i = 0; i < data.length; i++) {
      const value = data[i];
      if ((value & levelBit) === 0) {
        // this value goes to the left
        data[n++] = value;
      } else {
        bits.setOne(i);
        right.push(value);
      }
    }

    // append `right` to `data`, then clear `right`
    for (let i = 0; i < right.length; i++) {
      data[n++] = right[i];
    }
    right.length = 0;

    levels.push(new DenseBitVec(bits, rank1SamplesPow2, selectSamplesPow2));
  }

  // For the last level we don't need to do anything but build the bitvector
  {
    const bits = new BitBuf(data.length);
    const levelBit = 1;
    for (let i = 0; i < data.length; i++) {
      const value = data[i];
      if ((value & levelBit) !== 0) {
        bits.setOne(i);
      }
    }
    levels.push(new DenseBitVec(bits, rank1SamplesPow2, selectSamplesPow2));
  }

  return levels;
}