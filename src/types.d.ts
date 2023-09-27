interface BitVecBuilderConstructable {
  new(universeSize: number): BitVecBuilder;
}

interface BitVecBuilder {
  one(index: number, count?: number): void;
  build(options: object): BitVec;
}

// TODO: Split this trait into multiple traits.
// MultiBitVecs may not have rank0 or select0.
// Of course, they _do_ if they happen to not
// contain multiplicities, so maybe we can keep
// this simple and just do a runtime check.
interface BitVec {

  // Return the total count of 1-bits strictly below `index`.
  // In the case of multiplicity, each set 1-bit may contribute more than 1 unit of count.
  // In the case of a multiset, there may be multiple 1-bits at each index.
  rank1(index: number): number;
  rank0(index: number): number;

  select1(n: number): number;
  select0(n: number): number;

  maybeSelect1(n: number): number | null;
  maybeSelect0(n: number): number | null;

  get(index: number): number;

  // todo: sizeInBits()
  // todo: batchRank0/1 and batchSelect0/1

  readonly numOnes: number;
  readonly numZeros: number;

  readonly universeSize: number;
}

