class H2Histogram {
  // todo: constructor should probably take data, rather than these pieces
  // todo: Verify that all of the comments are correct in light of the fact that this histogram works differently than the previous.
  constructor(encoding, bins, counts) {
    assert(bins.length === counts.length);
    // assert no duplicates - or are duplicates fine (if inefficient)?

    const cumulativeCounts = new Float64Array(counts);
    for (let i = 1; i < cumulativeCounts.length; i++) {
      cumulativeCounts[i] += cumulativeCounts[i - 1];
    }

    this.bins = bins;
    this.cumulativeCounts = cumulativeCounts;
    this.encoding = encoding;
    this.numObservations =
      counts.length === 0 ? 0 : this.cumulativeCounts.at(-1);
  }

  // Return an upper bound on the number of observations at or below `value`.
  cumulativeCount(value) {
    // The index of the bin containing `value`
    const bin = this.encoding.binIndex(value);

    const i = Math.min(
      partitionPoint(this.bins.length, (i) => this.bins[i] < bin),
      this.bins.length
    );

    // The number of observations that are in or below that bin
    return this.cumulativeCounts[i];
  }

  // Like cumulative_count, but returns the fraction of the data rather than a count.
  cdf(value) {
    if (this.numObservations === 0) {
      return 1.0;
    }
    return this.cumulativeCount(value) / this.numObservations;
  }

  // Return an upper bound on the value of the q-th quantile.
  // Returns zero if the histogram contains no observation.
  quantile(q) {
    if (this.numObservations === 0) {
      return 0;
    }

    // Number of observations at or below the q-th quantile
    const k = this.quantileToCount(q);
    // this.bins[i] is the index of the bin containing the k-th observation.
    // There are two levels of indexing here, since `bins` itself contains "indices"
    const i = Math.min(
      partitionPoint(
        this.cumulativeCounts.length,
        (i) => this.cumulativeCounts[i] < k
      ),
      this.cumulativeCounts.length - 1
    );

    // Maximum value in that bin
    
    return this.encoding.highest(this.bins[i]);
  }

  // todo: this feels subtle - test it well, and maybe revise the implementation...
  /// Return an upper bound on the number of observations that lie
  /// at or below the q-th quantile. E.g. if there are 2 observations,
  /// quantile_to_count(0) == 1, quantile_to_count(0.25) == 1,
  /// quantile_to_count(0.75) == 2, quantile_to_count(1.0) == 2
  /// todo: clarify this docstring - this returns a value in [1, count]
  // todo: call it quantile to rank?
  quantileToCount(q) {
    DEBUG && assert(0.0 <= q && q <= 1.0);
    if (q == 0.0) {
      return 1;
    }
    return Math.ceil(q * this.numObservations);
  }
}


// constraint: no more than 2^32-1 bins
class H2Encoding {
  // Construct with either {m, r, n} or {a, b, n}
  constructor({ a, b, m, r, n }) {
    // Determine which of the two parameterizations – (a, b) or (m, r) –
    // this histogram is being constructed with.
    const abParameterization =
      a !== undefined && b !== undefined && m === undefined && r === undefined;
    const mrParameterization =
      a === undefined && b === undefined && m !== undefined && r !== undefined;
    assert(
      abParameterization || mrParameterization,
      "expected to be constructed with one of {m, r, n} or {a, b, n} parameters"
    );
    if (mrParameterization) {
      // in the classical (m, r) parameterization, m = a and r = c, which implies that b = r - m - 1.
      a = m;
      b = r - m - 1;
    }
    assertSafeInteger(n);
    assertSafeInteger(a);
    assertSafeInteger(b);

    assert(n <= 53);

    const c = a + b + 1;
    assert(c < 32);

    const numBins = H2Encoding.numBinsForParams(a, b, n);

    assert(numBins < 2 ** 32, `the number of bins may not exceed 2^32 - 1`);
    this.a = u32(a);
    this.b = u32(b);
    this.c = u32(c);
    this.n = u32(n);
    this.numBins = u32(numBins);
  }

  // Return the bin index of the value, given this histogram's parameters.
  // Values can be any number (including non-integers) within the value range.
  binIndex(value) {
    // Since JS numbers are 64-bit floats, we allow non-integral inputs.
    const { a, b, c } = this;
    assert(
      value >= 0 && value <= this.maxValue(),
      "expected value in histogram range [0, 2^n)"
    );

    if (value < u32(1 << c)) {
      // We're below the cutoff.
      // The bin width below the cutoff is 1 << a and we can use a bit shift
      // to compute the bin since we know the value is less than 2^32.
      return value >>> a;
    } else {
      // We're above the cutoff.

      // The log segment containing the value
      const v = Math.floor(Math.log2(value));

      // The bin offset within the v-th log segment.
      // To compute this with bit shifts: (value - u32(1 << v)) >>> (v - b)
      // - `value - (1 << v)` zeros the topmost (v-th) bit.
      // - `>>> (v - b)` extracts the top `b` bits of the value, corresponding
      //   to the bin index within the v-th log segment.
      //
      // To account for larger-than-32-bit inputs, however, we do this without bit shifts:
      const binsWithinSeg = Math.floor((value - 2 ** v) / 2 ** (v - b));
      DEBUG && assertSafeInteger(binsWithinSeg);

      // We want to calculate the number of bins that precede the v-th log segment.
      // 1. The linear section below the cutoff has twice as many bins as any log segment
      //    above the cutoff, for a total of 2^(b+1) = 2*2^b bins below the cutoff.
      // 2. Above the cutoff, there are `v - c` log segments before the v-th log segment,
      //    each with 2^b bins, for a total of (v - c) * 2^b bins above the cutoff.
      // Taken together, there are (v - c + 2) * 2^b bins preceding the v-th log segment.
      // Since the number of bins is always less than 2^32, this can be done with bit ops.
      const binsBelowSeg = u32((2 + v - c) << b);

      return binsBelowSeg + binsWithinSeg;
    }
  }

  // Given a bin index, returns the lowest value that bin can contain.
  lowest(binIndex) {
    const { a, b, c } = this;
    const binsBelowCutoff = u32(2 << b);
    if (binIndex < binsBelowCutoff) {
      return u32(binIndex << a);
    } else {
      // the number of bins in 0..binIndex that are above the cutoff point
      const n = binIndex - binsBelowCutoff;
      // the index of the log segment we're in: there are `c` log
      // segments below the cutoff and `n >> b` above, since each
      // one is divided into 2^b bins.
      const seg = c + (n >>> b);
      // by definition, the lowest value in a log segment is 2^seg
      // do this without bit shifts, since those return a 32-bit signed integer.
      const segStart = 2 ** seg;
      // the bin we're in within that segment, given by the low bits of n:
      // the bit shifts remove the `b` lowest bits, leaving only the high
      // bits, which we then subtract from `n` to keep only the low bits.
      const bin = n - u32((n >>> b) << b);
      // the width of an individual bin within this log segment (segStart >>> b)
      const binWidth = Math.floor(segStart / 2 ** b);
      // the lowest value represented by this bin is simple to compute:
      // start where the logarithmic segment begins, and increment by the
      // linear bin index within the segment times the bin width.
      return segStart + bin * binWidth;
    }
  }

  // Given a bin index, returns the highest value that bin can contain.
  highest(binIndex) {
    DEBUG && assert(binIndex < this.numBins);
    if (binIndex === this.numBins - 1) {
      return this.maxValue();
    } else {
      return this.lowest(binIndex + 1) - 1;
    }
  }

  // Return the bin width of the given bin index.
  binWidth(binIndex) {
    assert(binIndex < this.numBins);
    return this.highest(binIndex) - this.lowest(binIndex) + 1;
  }

  // Return the maximum value representable by these histogram parameters.
  maxValue() {
    return 2 ** this.n - 1;
  }

  absoluteError() {
    return 2 ** this.a;
  }

  relativeError() {
    return 2 ** -this.b;
  }

  cutoff() {
    return 2 ** this.c;
  }

  // Return the values of the (m, r) parameters, computing them from the stored (a, b).
  mr() {
    // (a, b) is used in the implementation, and (m, r) is returned
    // for people who are more familiar with that parameterization.
    const { a, b, n } = this;
    const m = a;
    const r = b - m - 1;
    return { m, r };
  }

  static numBinsForParams(a, b, n) {
    const c = a + b + 1;
    // todo: should this check that the number of bins is a safe integer?
    if (n < c) {
      // Each log segment is covered by bins of width 2^a and there are n log segments,
      // giving us 2^(n - a) bins in total. Also, we always want a minimum of 1 bin.
      return 2 ** Math.max(n - a, 0);
    } else {
      // See the comment about `binsBelowSeg` in `binIndex` for a derivation
      return (2 + n - c) * 2 ** b;
    }
  }
}