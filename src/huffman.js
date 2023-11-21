// Returns huffman-like optimal prefix-free codes that are optimized for use with a wavelet matrix.
// Short codes will sort to the end of the their lowest level in the wavelet matrix, ensuring that
// the "gaps" they introduce into the full binary tree modeled by the wavelet matrix do not introduce
// any gaps between codes at lower levels, since all longer codes are guaranteed to precede them.
//
// This is ensured by observing that the wavelet matrix sorts nodes in ascending order of their bit-reversed
// symbol (unlike the wavelet tree, which sorts nodes in ascending order by symbol with no bit-reversal).
//
// These codes preserve the ability to navigate the wavelet matrix by using bitvector rank operations since
// you can rely on the values being correct even in the face of elements that do not appear at the lower
// levels, since these "disappearing" elements (represented by shorter codes on the higher levels) are
// always at the end of the matrix, so their disappearance does not introduce "missing" symbols in the tree
// that would cause the ranks to report inaccurate counts.
//
// Works levelwise, generating short codes before long codes. As we go from level to level we maintain
// a sorted array C of available codes and pick the required number of codes off for each level as we go,
// taking a sorted subslice of the required length from the end of C.
//
// After we take the codes for a given level, we generate two new codes from each remaining code in C,
// by appending a 0-bit and a 1-bit, respectively, while ensuring the sorted order of C is maintained.
//
// Note that the return to codes may contain what appear to be duplicates. These are not actually
// duplicate codes: their code lengths are different (one or both are padded on the high end with zeros).
//
// This algorithm is described in the paper "The wavelet matrix: An efficient wavelet tree for large
// alphabets" and also discussed in the paper "Practical Wavelet Tree Construction", which also talks
// about how to adapt wavelet matrix construction algorithms to work with variable-length codes.
//
// The input parameter `l` is an array of the desired code lengths, sorted in ascending order.
/**
 * @param {number[]} l
 */
export function waveletMatrixCodes(l) {
  if (l.length === 0) return [];

  // maximum code length
  const L = l[l.length - 1];

  // `hist` is a histogram of the code lengths
  let hist = new Uint32Array(L + 1);
  for (const len of l) {
    hist[len]++;
  }

  let codes = [];
  let C = [0, 1];
  for (let level = 1; level <= L; level++) {
    // C[split..] contains the codes to be emitted at this level, in ascending order by bit-reversed value.
    // Emitting them this way means that our codes are assigned in increasing order in the wavelet matrix
    // at each level
    let split = C.length - hist[level];
    for (let i = split; i < C.length; i++) {
      codes.push(C[i]);
    }

    // truncate the array to include only the remaining codes
    C.length = split;

    // for each remaining code, append a 0-bit to it, then push a version of the code with a 1-bit
    // appended to it. Since the original codes were ordered in ascending order by bit-reversed value,
    // the codes in C[0..split] remain sorted. Similarly, since the appended 1-bit causes all of those
    // codes to come after their 0-appended counterparts, they too are in sorted order. And since we
    // append the 1-suffixed codes to the 0-suffixed codes, these two groups are also in sorted order.
    // So this is a cheap way to maintain the sort order for the next level iteration.
    // We guard this by an `if` to not generate codes that we will not need in the final iteration.
    if (level === L) break;

    for (let i = 0; i < split; i++) {
      C[i] *= 2;
      C.push(C[i] + 1);
    }
  }
  return codes;
}

// Compute Huffman codeword lengths, in-place linear-time version.
// The input parameter `weights` is the list of weights, in decreasing-weight order.
// Implements Algorithm 2 from Alistair Moffat's 2019 article, "Huffman Coding".
// See Section 2.4 of the article for an explanation and illustration of the algorithm,
// which uses the W array to store, at different times, the input weights, weights of
// internal nodes, parent locations of internal nodes, internal node depths, and leaf depths.
// The parameter `weights` should be an array of integer or floating-point weights, sorted in
// descending order. One interpretation of the weights is as symbol frequencies. Higher-frequency
// elements will be assigned shorter codewords.
/**
 * @param {number[]} weights
 */
export function huffmanCodeLengths(weights) {
  // don't mutate the original input
  // (we could mutate it if we wanted to save space, but it's unconventional to do so in JS)
  const W = weights.slice();
  // Phase 1
  let n = W.length;
  let leaf = n - 1;
  let root = n - 1;
  for (let next = n - 1; next > 0; next--) {
    // find first child
    if (leaf < 0 || (root > next && W[root] < W[leaf])) {
      // use internal node
      W[next] = W[root];
      W[root] = next;
      root--;
    } else {
      // use leaf node
      W[next] = W[leaf];
      leaf--;
    }

    // same as above, but adding to W[next] rather than assigning to it
    if (leaf < 0 || (root > next && W[root] < W[leaf])) {
      W[next] += W[root];
      W[root] = next;
      root--;
    } else {
      W[next] += W[leaf];
      leaf--;
    }
  }

  // Phase 2
  W[1] = 0;
  for (let next = 2; next < n; next++) {
    W[next] = W[W[next]] + 1;
  }

  // Phase 3
  let avail = 1;
  let used = 0;
  let depth = 0;
  let next = 0;
  root = 1;
  while (avail > 0) {
    // count internal nodes used at depth `depth`
    while (root < n && W[root] === depth) {
      used++;
      root++;
    }

    // assign as leaves any nodes that are not internal
    while (avail > used) {
      W[next] = depth;
      next++;
      avail--;
    }
    // move to next depth
    avail = 2 * used;
    depth++;
    used = 0;
  }

  // W[i] now contains the length of the i-th codeword
  return W;
}

// Generate a list of lexicographically-ordered codewords from non-decreasing array of Huffman codeword lengths specified by `lengths`.
//
// The returned codes will be of increasing value (lexicographically ordered) and nondecreasing code length.
// See Section 2.5: Assigning Codewords of Alistair Moffat's 2019 article, "Huffman Coding".
/**
 * @param {number[]} lengths
 */
export function canonicalHuffmanCodes(lengths) {
  const n = lengths.length;
  if (n === 0) return [];
  const L = lengths[n - 1];
  let code = 0;
  const codes = new Uint32Array(n);
  for (let i = 1; i < n; i++) {
    code += 2 ** (L - lengths[i - 1]);
    codes[i] = code;
  }
  for (let i = 1; i < n; i++) {
    codes[i] >>>= L - lengths[i];
  }
  return codes;
}