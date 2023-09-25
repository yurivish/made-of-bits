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
  rank0(index: number): number;
  rank1(index: number): number;

  select0(n: number): number;
  select1(n: number): number;

  maybeSelect0(n: number): number | null;
  maybeSelect1(n: number): number | null;

  get(index: number): number;

  // todo: sizeInBits()
  // todo: batchRank0/1 and batchSelect0/1

  readonly numOnes: number;
  readonly numZeros: number;

  readonly universeSize: number;
}

