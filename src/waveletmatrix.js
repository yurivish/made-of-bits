import { assert } from './assert.js';
import { BitBuf } from './bitbuf.js';
import { oneMask, reverseLowBits, u32 } from './bits.js';
import { DenseBitVec } from './densebitvec.js';

class WaveletMatrix {

  /**
   * @param {number[]} data
   * @param {number} maxSymbol
   */
  constructor(data, maxSymbol) {
    assert(maxSymbol < 2 ** 32);
    const alphabetSize = maxSymbol + 1;
    const numLevels = Math.max(1, Math.ceil(Math.log2(alphabetSize)));
    // We implement two different wavelet matrix construction algorithms. One of them is more
    // efficient, but that algorithm does not scale well to large alphabets and also cannot
    // cannot handle element multiplicity because it constructs the bitvectors out-of-order.
    // It also requires O(2^num_levels) space. So, we check whether the number of data points
    // is less than 2^num_levels, and if so use the scalable algorithm, and otherise use the
    // the efficient algorithm.
    /** @type {BitVec[]} */
    let bitVecs;
    if (data.length === 0) {
      // Create an empty bitvec since numLevels is 1
      bitVecs = [new DenseBitVec(new BitBuf(0), 5, 5)];
    } else if (numLevels <= Math.floor(Math.log2(data.length))) {
      bitVecs = buildBitVecsSmallAlphabet(data, numLevels);
    } else {
      bitVecs = buildBitVecsLargeAlphabet(data, numLevels);
    };

    this.initFromBitVecs(bitVecs);
  }

  /**
   * @param {BitVec[]} bitVecs
   */
  initFromBitVecs(bitVecs) {
    assert(bitVecs.length > 0);
    this.numLevels = bitVecs.length;
    this.maxLevel = this.numLevels - 1;
    this.length = bitVecs[0].universeSize;
    this.levels = bitVecs.map((bv, index) => ({
      nz: bv.numZeros,
      bit: u32(1 << (bitVecs.length - index)),
      bv
    }));
  }
}

/**
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
    for (let i = 0; i < numNodes; i++) {
      // Update the positions in-place. The bit reversals map from wavelet tree
      // node order to wavelet matrix node order, with all left children preceding
      // the right children.
      let prev_index = reverseLowBits(i - 1, l);
      borders[reverseLowBits(i, l)] = borders[prev_index] + hist[prev_index];
    }

    // Fill the bit vector of the current level
    const level = levels[l];
    const levelBitIndex = maxLevel - l;
    const levelBit = 1 << levelBitIndex;

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
  return levels.map(d => new DenseBitVec(d, 5, 5));
}

/**
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

    // append right to data then clear it
    for (let i = 0; i < right.length; i++) {
      data[n++] = right[i];
    }
    right.length = 0;

    levels.push(new DenseBitVec(bits, 5, 5));
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
    levels.push(new DenseBitVec(bits, 5, 5));
  }

  return levels;
}