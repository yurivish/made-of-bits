interface BitVecBuilderConstructable {
  // todo: consider number | undefined for the constructor,
  // with undefined indicating that the maximum 1-bit position
  // should determine the universe size (or 0 if no 1-bits).
  new(universeSize: number): BitVecBuilder;
}

interface BitVecBuilder {
  // The second argument is optional and customarily filled in with a default value of 1.
  one(index: number, count?: number): void;
  build(options: object): BitVec;
}

// TODO: Split this trait into multiple traits.
// MultiBitVecs may not have rank0 or select0.
// Of course, they _do_ if they happen to not
// contain multiplicities, so maybe we can keep
// this simple and just do a runtime check.
interface BitVec {

  // Return the total count of 1-bits strictly below the`index`.
  // In the case of multiplicity, each set 1-bit may contribute more than 1 unit of count.
  // In the case of a multiset, there may be multiple 1-bits at each index.
  rank1(index: number): number;
  rank0(index: number): number;

  select1(n: number): number;
  select0(n: number): number;

  trySelect1(n: number): number | null;
  trySelect0(n: number): number | null;

  get(index: number): number;

  // todo: approxSizeInBytes() // ignoring fixed-width fields
  // todo: batchRank0/1 and batchSelect0/1

  readonly numOnes: number;
  readonly numZeros: number;

  // todo: this comment was pasted in from Rust; adapt/modify/remove it.
  // num_ones() + num_zeros() in the non-multiplicity case
  // Note: Since `len` returns a value of type `Ones`,
  // the maximum length of a BitVec is 2^n-1 and the
  // maximum index is 2^n-2, with n = Ones::BITS.
  // This means that you cannot have a BitVec with its
  // (2^n-1)-th bit set even though that value is
  // representable by the Ones type (it is Ones::MAX).
  // This is a trade-off in favor of sanity: if we
  // allowed BitVecs of length 2^n, then there could
  // be 2^n 0-bits or 1-bits in an array, and all of
  // the relevant functions would need to use higher
  // bit widths for their return values and internal
  // computations. So we opt for sanity at the low level
  // and can compensate at higher levels if needed (e.g.
  // by storing the count of elements in the phantom
  // (2^n-1)-th position separately and perhaps using
  // a rank1p function that is analogous to log1p,
  // which would compute rank1(i+1) and work even when
  // i+1 and the resulting rank would exceed the bit width
  // of Ones.
  readonly universeSize: number;

  readonly hasMultiplicity: boolean;

  // these differ from their non-unique counterparts 
  // in the presence of multiplicity
  readonly numUniqueOnes: number;
  readonly numUniqueZeros: number;

}

