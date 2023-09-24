interface BitVecBuilderConstructable {
  new(universeSize: number): BitVecBuilder;
}

interface BitVecBuilder {
  one(index: number, count?: number): void;
  build(options: object): BitVec;
}

interface BitVec {
  rank0(index: number): number;
  rank1(index: number): number;

  select0(n: number): number;
  select1(n: number): number;

  maybeSelect0(n: number): number | null;
  maybeSelect1(n: number): number | null;

  get(index: number): number;

  readonly numOnes: number;
  readonly numZeros: number;
  readonly universeSize: number;
}

