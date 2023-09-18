// Dense bit vector with rank and select, based on the ideas described in
// the paper "Fast, Small, Simple Rank/Select on Bitmaps".
// We use an additional level of blocks provided by BitVec, but the ideas are the same.

// todo:
// - transfer more comments from the rust code
// - is it weird that all the "BLOCK_BITS" functions are in "bits"
//   and not bitbuf? Isn't it the bit buf that actually has the blocks?
//   It only makes sense if other types use the same blocks, and I guess
//   the IntBuf also uses them. But this should be clearly documented & explained!
// - note the precise meaning of s0 and s1. Does each select sample point at the block containing its bit?
// - note the precise meaning of r; is it the number of ones *preceding* this block?
// - fix all snake_case to camelCase
// - call this RankSelect since it does not store the data itself (rather accepts a bitbuf)
//   but augments it with fast rank/select indexes?
// - i kinda wanna visualize when select samples are taken – right now I don't really understand it...
//   and this would be useful for a writeup of this technique.
// - decide whether to use the Pow2 suffix or the _LOG2 suffix or something else, but be consistent with both
// - consistently order Select0 and Select1 handling (probably 1 before 0 since rank1/select1 pull focus)
// - Investigate using u32 to also type case to a nominal U32 type.
// - consider calling them Rank1Samples
// - for select1/select0, explore binary search over the suffix of rank samples:
//   we want the rightmost rank index such that r1[rankIndex] <= n.
// - exciting question: what could it look like to do batch-select on a sorted set of ns?
//   - maybe transducers are involved
//   - figure out how to access each piece of data just once - eg. never re-access a rank block.
//   - i think nested for loop where each inner loop increments ns[i] while it is within the same type of block.
//   - we can check times against simple-sds: https://github.com/jltsiren/simple-sds
//     - it has bv-benchmark and wm-venchmark: https://github.com/jltsiren/simple-sds/blob/main/src/bin/bv-benchmark/main.rs
// - read this good post on array performance in v8: https://v8.dev/blog/elements-kinds

import { DEBUG, assert, assertNotUndefined, assertSafeInteger, log } from "./assert.js";
import { BitBuf } from './bitbuf.js';
import * as bits from './bits.js';

export class DenseBitVec {
  /**
   * @param {BitBuf} data - bit buffer containing the underlying bit data
   * @param {number} srPow2 - power of 2 of the rank sample rate
   * @param {number} ssPow2 - power of 2 of the select sample rate
   */
  constructor(data, srPow2, ssPow2) {
    // todo: 
    // - kw args for sampling rates, with 2^10 being default
    // - Accept s0Pow2, s1Pow2 instead of ssPow2 in order to control the space usage; 
    //   the s0 index only matters for select0, while select1 helps speed up rank1 and rank0.
    // - document the meanings of the values of the r/s0/s1 arrays.
    // - update all references to 'raw' blocks – those mean the blocks in the bitbuf.
    // - assert there are less than 2^32 bits (since we do bitwise ops on bit indexes)
    assertSafeInteger(srPow2); 
    assertSafeInteger(ssPow2);
    assert(srPow2 >= bits.BLOCK_BITS_LOG2, 'sr must be a positive multiple of the block size');
    assert(ssPow2 >= bits.BLOCK_BITS_LOG2, 'ss must be a positive multiple of the block size');

    const ss1 = 1 << ssPow2; // Select1 sampling rate: sample every `ss1` 1-bits
    const ss0 = 1 << ssPow2; // Select0 sampling rate: sample every `ss0` 0-bits
    const sr1 = 1 << srPow2; // Rank sampling rate: sample every `sr` bits

    // Distinguish
    // - Which bit (position) the sample represents
    // - What it stores about that (or related) bit positions
    // - Bits vs blocks; we want our stuff block-aligned

    // Each rank sample identifies a particular basic block. 
    // 

    // Rank samples are sampled every `rankSamplingRate` bits, where `rankSamplingRate` is a positive multiple of
    // the bit width of a basic block. For example, if `rankSamplingRate` is 64 and the basic
    // block width is 32, then the rank samples will tell us about the 0th, 2nd, 4th, 6th, ... basic block.
    //
    // A rank sample `r[i]` tells us about the basic block `data.blocks[i << (srPow2 - bits.BLOCK_BITS_LOG2)]`.
    //
    // If `r[i] has value `v`, this means that there are `v` 1-bits preceding that basic block.
    // Rank samples represent the number of 1-bits up to but not including a basic block.
    // todo: we could preallocate a Uint32Array since we know the number of rank samples in advance
    // todo: we could preallocate
    const r = []; // rank samples

    // Each select1 sample identifies a particular basic block.
    //
    // Select samples are sampled every `select1SampleRate` 1-bits, where `rankSamplingRate` is a positive multiple of
    // the bit width of a basic block. Unlike rank blocks, which start sampling from 0 (representing the 
    // `rankSamplingRate*i + 0`-th bits), select blocks start sampling from 1, and thus represent the
    // `select1SamplingRate*i + 1`-th bits.
    // For example, if `select1SamplingRate` is 64, then the select1 samples will identify the basic blocks
    // that contain the 0+1 = 1st, 64+1 = 65th, 2*64+1 = 129th, 3*64+1 = 193rd, ... bits. Note that since
    // the sampling rate is a positive multiple of the basic block, two select block will never point 
    // to the same basic block.
    const s1 = []; // select1 samples
    const s0 = []; // select0 samples

    // Select1 samples represent the number of 1-bits up to but not including a basic block.
    // For example, if `select1SamplingRate`
    // is 64, then the select1 samples will tell us about the basic blocks containing the 1st
    // A select sample `s1[i]` tells us about the basic block that contains the
    // `selectSamplingRate * i + 1`-th 1-bit.

    let cumulativeOnes = 0; // 1-bits preceding the current raw block
    let cumulativeBits = 0; // bits preceding the current raw block
    let zerosThreshold = 0; // take a select0 sample at the (zerosThreshold+1)th 1-bit
    let onesThreshold = 0; // take a select1 sample at the (onesThreshold+1)th 1-bit

    const basicBlocksPerRankSample = sr1 >>> bits.BLOCK_BITS_LOG2;
    const blocks = data.blocks;

    const maxBlockIndex = blocks.length - 1;
    for (let blockIndex = 0; blockIndex < blocks.length; blockIndex++) {
      const block = blocks[blockIndex];
      if (blockIndex % basicBlocksPerRankSample === 0) {
        r.push(cumulativeOnes);
      }

      const blockOnes = bits.popcount(block);
      let blockZeros = bits.BLOCK_BITS - blockOnes;
      // Don't count trailing zeros in the final data block towards the zero count
      if (blockIndex === maxBlockIndex) blockZeros -= data.numTrailingZeros;
      const cumulativeZeros = cumulativeBits - cumulativeOnes;


      // Sample 1-bits for the select1 index
      if (cumulativeOnes + blockOnes > onesThreshold) {
        // Take a select1 sample, which consists of two parts:
        // 1. The cumulative number of bits preceding this basic block, ie. the left-shifted block index.
        //    This is `cumulativeBits`, defined above, and is stored in the high bits.
        // 2. A correction factor storing the number of 1-bits preceding the (ss1 * i + 1)-th 1-bit within this
        //    basic block, which we can use to determine the number of 1-bits preceding this basic block.
        //    Effectively, this is a way for us to store samples that are slightly offset from the strictly
        //    regular select sampling scheme, enabling us to keep the select samples aligned to basic blocks.
        //    This is `correction`, and is stored in the low bits.
        const correction = onesThreshold - cumulativeOnes;
        // Since cumulativeBits is a multiple of the basic block size,
        // these two values should never overlap in their bit ranges.
        DEBUG && assert((cumulativeBits & correction) === 0);
        // Add the select sample and bump the onesThreshold.
        s1.push(cumulativeBits | correction);
        onesThreshold += ss1;
      }

      // Sample 0-bits for the select0 index.
      // This `if` block has the same structure as the one above which samples 1-bits.
      if (cumulativeZeros + blockZeros > zerosThreshold) {
        const correction = zerosThreshold - cumulativeZeros;
        DEBUG && assert((cumulativeBits & correction) === 0);
        s0.push(cumulativeBits | correction);
        zerosThreshold += ss0;
      }

      cumulativeOnes += blockOnes;
      cumulativeBits += bits.BLOCK_BITS;
    }

    /** @readonly */
    this.data = data;

    /** @readonly */
    this.srPow2 = srPow2;

    /** @readonly */
    this.s0Pow2 = ssPow2;

    /** @readonly */
    this.s1Pow2 = ssPow2;

    /** @readonly */
    this.rankSamples = new Uint32Array(r);

    /** @readonly */
    this.s0 = new Uint32Array(s0);

    /** @readonly */
    this.s1 = new Uint32Array(s1);

    /** @readonly */
    this.basicBlocksPerRankSamplePow2 = srPow2 - bits.BLOCK_BITS_LOG2;

    /** @readonly */
    this.numOnes = cumulativeOnes;

    /** @readonly */
    this.numZeros = data.lengthInBits - cumulativeOnes;

    // todo: call this 'universe size' for compatibility with multibitvecs?
    /** @readonly */
    this.lengthInBits = data.lengthInBits;

    
  }

  /**
   * @param {number} n
   */
  select1(n) {
    const result = this.maybeSelect1(n);
    if (result === null) throw new Error(`n ${n} is not a valid 1-bit index`);
    return result;
  }

  // TODO: maybe select w/ rankSampler /* abstract the rank sampling so we can use the same f for select and 1 */
  // tricky to generalize the below - since we need to replace the rank sampling, rank length checking,
  // as well as the popcounting

  /**
   * @param {number} n
   */
  // prevMaybeSelect1(n) {
  //   // We're looking for the bit index of the n-th 1-bit.
  //   // If there is no n-th 1-bit, then return null.
  //   if (n < 0 || n >= this.numOnes) return null; // throw new Error('n is not a valid 1-bit index');

  //   // Find the closest preceding select block to the n-th 1-bit
  //   const s = this.select1Sample(n); // note: interesting that we do not use precedingCount.

  //   let count = 0;
  //   // Search forward until the next rank sample exceeds n, which indicates that the current
  //   // rank sample represents the range of basic blocks containing the n-th bit.
  //   // We iterate in a slightly subtle way in order to minimize the number of memory accesses.
  //   let rankIndex = s.basicBlockIndex >>> this.basicBlocksPerRankSamplePow2;
  //   DEBUG && assert(rankIndex < this.r1.length); // so the loop below runs at least once
  //   while (rankIndex < this.r1.length) {
  //     let nextCount = this.r1[rankIndex];
  //     if (nextCount > n) break;
  //     count = nextCount; 
  //     rankIndex++;
  //   }
  //   // Each rank sample's value indicates the number of _preceding_ 1-bits. By the time
  //   // we break out of the loop, we have incremented the index one too far. So, rewind.
  //   rankIndex--;

  //   // Find the basic block containing the n-th 1-bit.
  //   const blocks = this.data.blocks;
  //   let basicBlock = 0;
  //   let basicBlockIndex = rankIndex << this.basicBlocksPerRankSamplePow2;
  //   DEBUG && assert(basicBlockIndex < blocks.length); // so the loop below runs at least once
  //   while (basicBlockIndex < blocks.length) {
  //     basicBlock = blocks[basicBlockIndex];
  //     const nextCount = count + bits.popcount(basicBlock);
  //     if (nextCount > n) break;
  //     count = nextCount;
  //     basicBlockIndex++;
  //   }

  //   // index of the start of the basic block, and bit offset within the basic block
  //   const blockBitIndex = basicBlockIndex << bits.BLOCK_BITS_LOG2;
  //   const bitOffset = bits.select1(basicBlock, n - count);
  //   return blockBitIndex + bitOffset;
  // }


  /**
   * Foo.
   * @param {number} n - blah.
   * # Implementation notes
   * - We set `basicBlockIndex` inside the rank sample loop rather than immediately after it
   *   in order to avoid ever setting it to a value less than its initial value. This could
   *   Otherwise, occur if the very first sampled rank block. 
   * - The special case where we are looking for the exact bit pointed to by the select block
   *   will incur rank scans here, since this special case is unlikely in the general case as
   *   there are many more bits that are not select samples.
   */
  maybeSelect1(n) {
    if (n < 0 || n >= this.numOnes) return null;

    let { basicBlockIndex, precedingCount: count } = this.select1Sample(n);
    assert(count <= n);

    if (DEBUG) {
      const prevRankIndex = basicBlockIndex >>> this.basicBlocksPerRankSamplePow2;
      assert(prevRankIndex < this.rankSamples.length);
    }

    let rankIndex = (basicBlockIndex >>> this.basicBlocksPerRankSamplePow2) + 1;
    while (rankIndex < this.rankSamples.length) {
      let sample = this.rankSamples[rankIndex];
      if (sample > n) break;
      count = sample;
      basicBlockIndex = rankIndex << this.basicBlocksPerRankSamplePow2;
      rankIndex++;
    }
    
    // traverse across basic blocks until we find the block containing the n-th 0-bit
    let basicBlock = 0;
    assert(basicBlockIndex < this.data.blocks.length); // the loop runs at least once
    while (basicBlockIndex < this.data.blocks.length) {
      basicBlock = this.data.blocks[basicBlockIndex];
      const nextCount = count + bits.popcount(basicBlock);
      if (nextCount > n) break;
      count = nextCount;
      basicBlockIndex++;
    }

    // compute and return the bit index of the n-th 0-bit
    const blockBitIndex = basicBlockIndex << bits.BLOCK_BITS_LOG2;
    const bitOffset = bits.select1(basicBlock, n - count);
    return blockBitIndex + bitOffset;
  }

  /**
   * @param {number} n
   */
  maybeSelect0(n) {
    if (n < 0 || n >= this.numZeros) return null;

    let { basicBlockIndex, precedingCount: count } = this.select0Sample(n);
    assert(count <= n);

    // use rank samples to traverse across basic blocks more efficiently
    let rankIndex = basicBlockIndex >>> this.basicBlocksPerRankSamplePow2;
    assert(rankIndex < this.rankSamples.length);
    while (rankIndex < this.rankSamples.length) {
      let nextCount = (rankIndex << this.s0Pow2) - this.rankSamples[rankIndex];
      if (nextCount > n) break;
      basicBlockIndex = rankIndex << this.basicBlocksPerRankSamplePow2;
      count = nextCount;
      rankIndex++;
    }

    // traverse across basic blocks until we find the block containing the n-th 0-bit
    let basicBlock = 0;
    assert(basicBlockIndex < this.data.blocks.length);
    while (basicBlockIndex < this.data.blocks.length) {
      basicBlock = this.data.blocks[basicBlockIndex];
      const nextCount = count + bits.popcount(~basicBlock);
      if (nextCount > n) break;
      count = nextCount;
      basicBlockIndex++;
    }

    // compute and return the bit index of the n-th 0-bit
    const blockBitIndex = basicBlockIndex << bits.BLOCK_BITS_LOG2;
    const bitOffset = bits.select1(~basicBlock, n - count);
    return blockBitIndex + bitOffset;
  }

  /**
   * @param {number} n
   */
  select0(n) {
    const result = this.maybeSelect0(n);
    if (result === null) throw new Error('n is not a valid 0-bit index');
    return result;
  };


  /**
   * @param {number} n
   */
  select1Sample(n) {
    DEBUG && assert(n < this.numOnes);
    return this.selectSample(n, this.s1, this.s1Pow2);
  }

  /**
   * @param {number} n
   */
  select0Sample(n) {
    DEBUG && assert(n < this.lengthInBits - this.numOnes);
    return this.selectSample(n, this.s0, this.s0Pow2);
  }

  /**
   * @param {number} n - we are looking for the n-th bit of the particular kind (1-bit or 0-bit)
   * @param {number} sr - power of 2 of the sample rate
   * @param {Uint32Array} samples - array of samples
   */
  selectSample(n, samples, sr) {
    const sampleIndex = n >>> sr;
    const sample = samples[sampleIndex];

    // bitmask with the Raw::BLOCK_BITS_LOG2 bottom bits set.
    const mask = bits.BLOCK_BITS - 1;
    
    // The cumulative number of bits preceding the identified basic block, 
    // ie. the left-shifted block index of that block.
    const cumulativeBits = sample & ~mask; // high bits

    // NOTE: The references to 1-bits below are written from the perspective of select1.
    // If using this function for select zero, every instance of 1-bit should be replaced by 0-bit.

    // The number of 1-bits in the identified basic block preceding the (select1SampleRate*i+1)-th 1-bit
    const correction = sample & mask; // low bits

    // number of 1-bits preceding the identified basic block.
    // The first term tells us the number of 1-bits preceding this select sample,
    // since the k-th sample represents the (k*sr + 1)-th bit and this tells us the (k*sr)-th
    // The second term allows us to identify how may 1-bits precede the basic block containing
    // the bit identified by this select sample.
    const precedingCount = (sampleIndex << sr) - correction;

    return {
      basicBlockIndex: cumulativeBits >>> bits.BLOCK_BITS_LOG2,
      precedingCount
    };
  }

  // next:
  // x fn to decode a select block
  // x select1
  // - select0
  // - rank1
  // - rank0
  // - get
  // - numOnes
  // - universeSize
  // - get
}


 // todo: function to decode a single sample?

  // todo: document & explain what this does
/**
 * Returns the information contained in the closest select sample
 * preceding the n-th sampled bit [todo: reword... least upper bound?]
 * Eg. we are looking for n=50. We may have a select sample representing
 * the 30th s-bit, saying it is at position 12345. { bitIndex: 12345, numOnes: 30 }
 * @param {Uint32Array} s
 * @param {number} ssPow2
 * @param {number} n - the n-th (0|1)-bit
*/

// selectSample(s, ssPow2, n) {
//   // Select samples are taken every j*2^ssPow2 1-bits and stores
//   // a value related to the bit position of the 2^ssPow2-th bit.
//   // For improved performance, rather than storing the position of
//   // that bit directly, each select sample holds two separate values:
//   // 1. The raw-block-aligned bit position of that bit, ie. the number
//   //    of bits preceding the raw block containing the 2^ssPow2-th bit.
//   // 2. The bit position of the (ss * i + 1)-th 1-bit within that raw block,
//   //    which we can subtract from j*2^ss_pow2 to tell the number of 1-bits
//   //    up to the raw-block-aligned bit position.
//   const sampleIndex = n >> ssPow2;
//   const sample = s[sampleIndex];
//   return this.decodeSelectSample(sample, sampleIndex << ssPow2);
// }

// /**
//  * @param {number} sample - the select sample
//  * @param {number} sampleBitIndex - the bit index represented by the sample, ie. for s1[i] it is i << s1Pow2
//  */
// decodeSelect1Sample(sample) {
//   // bitmask with the Raw::BLOCK_BITS_LOG2 bottom bits set.
//   const lowMask = bits.BLOCK_BITS - 1;

//   // The cumulative number of bits preceding the identified basic block, ie. the left-shifted block index of that block.
//   const cumulativeBits = sample & ~lowMask; // high

//   // The number of 1-bits in the identified basic block preceding the (select1SampleRate*i+1)-th 1-bit
//   const correction = sample & lowMask; // low

//   // log(cumulativeBits, sampleBitIndex.toString(2), (~lowMask >>> 0).toString(2));
//   // assert(bitIndex === sampleBitIndex);

//   // assert that bit pos is data-block-aligned
//   DEBUG && assert(bits.blockBitOffset(cumulativeBits) === 0);

//   // Number of 1-bits represented by this sample, up to the raw block boundary
//   // todo: these are not necessarily ONES; s may represent 0-samples (or even "01"-samples in the future)
//   // todo: clarify if this is the count of 1-bits < bitIndex, or <= bitIndex
//   // I now think this is the number of 1-bits *preceding* bitIndex.
//   // But man this stuff is somehow very fiddly... I will feel better when I have a basic version passing tests,
//   // because then I can work on simplifying this code while preserving correct behavior.
//   const count = sampleBitIndex - correction;
//   // our bit index is block-aligned, so we can just return it as a block index. 
//   // todo: explain this more fully
//   // todo: remove the fields we do not use from here
//   return { bitIndex: cumulativeBits, dataBlockIndex: cumulativeBits >> bits.BLOCK_BITS_LOG2, rIndex: cumulativeBits >> this.srPow2, count };
// }


// The target bit is somewhere in the data buffer. 
// Use select samples to compute a lower bound on its position (bit index).
// There are `count` 1-bits up to but not including index `bitIndex`.
// let { bitIndex, dataBlockIndex, rIndex, count } = this.selectSample(this.s1, this.ssPow2, n);

// blockIndex (todo: rename for clarity) is a bitbuf block index.
// so shift by 
// log({ srPow2: this.srPow2, BLOCK_BITS_LOG2: this.BLOCK_BITS_LOG2 },);
// log('waaao', bitIndex, blockIndex, blockIndex >>> (this.srPow2 - bits.BLOCK_BITS_LOG2));
// assert(this.r[blockIndex << (this.srPow2 - bits.BLOCK_BITS_LOG2)] === count);
// // there are `count` 1-bits up to but not including `bitIndex`, which is a rank-block-aligned index.

// There may be some rank samples in between the lower bound and
// the true position; iterate over rank blocks until we find the
// last one whose count is less than or equal to n.

// Use rank blocks to hop over many raw blocks.
// This could use exponential search over [blockIndex, blocks.length);
// depending on the query and bit distribution this could be an improvement.
// Currently the worst case is linear search over rank samples.
// index of the next rank block
// const blocks = this.data.blocks;
// let lastKnownGood = rIndex;
// let lastKnownCount = count;
// rIndex++;
// while (rIndex < blocks.length && (count = this.r[rIndex]) < n) {
//   lastKnownGood = rIndex;
//   lastKnownCount = count;
//   rIndex++;
// }
// return (rIndex);
// for (let i = rIndex + 1; rIndex < blocks.length; i++) {
//   break;
// }

// // Find the rank block right before the one that exceeds n. Or the final block, if none exceeds it.
// while (blockIndex < blocks.length && blocks[blockIndex] < n) {
//   const nextCount = blocks[blockIndex];
//   if (nextCount < n) {
//     count = nextCount;
//     blockIndex++;
//   }
// }




  // /**
  //  * @param {number} bitIndex
  //  */
  // toBasicBlockIndex(bitIndex) {
  //   return bitIndex >> bits.BLOCK_BITS_LOG2;
  // }

  // /**
  //  * @param {number} bitIndex
  //  */
  // toRankSampleIndex(bitIndex) {
  //   return bitIndex >> this.srPow2;
  // }

  // /**
  //  * @param {number} bitIndex
  //  */
  // toSelect0SampleIndex(bitIndex) {
  //   return bitIndex >> this.s0Pow2;
  // }

  // /**
  //  * @param {number} bitIndex
  //  */
  // toSelect1SampleIndex(bitIndex) {
  //   return bitIndex >> this.s1Pow2;
  // }



// Select samples are of the (ss*i+1)-th bit. Eg. if ssPow2 = 2, we sample the 1, 1+4=5, 5+4=9, 9+4=13, 13+4=17-th bits.
// So the first select block points to the raw block containing the 1st bit.
// The second block points to the raw block containing the 5th bit.
// And it also stores correction info, since eg. maybe that raw block has 2 set bits in it before the 5th bit.
// In that case, the second block points to the raw block containing the 5th bit, and tells us there are 3 bits preceding it.
// 
// Yeah. I think I want a visualization of the state after this constructor has run.
// Which means bundling this and serving it to a notebook (maybe esbuild does cors)

// I wonder if types can help disentangle my confusion with regard to the varieties of indexes and block types that are floating around.
// The runtime nature of the type system might make it impossible to use a newtype-like pattern.

/*

The plan
  
  move to a concept of a 'basic block', which is the block size used by the fundamental bit types,
    bit buf and int buf.

  in this library,
    'buf' means just for reading/writing
    'vec' means 'bit vector' means rank/select
  

*/
