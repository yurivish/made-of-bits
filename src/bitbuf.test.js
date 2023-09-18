import { describe, expect, it, test } from 'vitest';
import { BitBuf } from './bitbuf.js';

describe('BitBuf', () => {
  const xs = new BitBuf(3);

  it('should be initialized to false', () => {

    expect(xs.get(0)).toBe(0);
    expect(xs.get(1)).toBe(0);
    expect(xs.get(2)).toBe(0);
  });

  it('should set bits individually', () => {
    xs.setOne(1);
    expect(xs.get(0)).toBe(0);
    expect(xs.get(1)).toBe(1);
    expect(xs.get(2)).toBe(0);

    xs.setOne(2);
    expect(xs.get(0)).toBe(0);
    expect(xs.get(1)).toBe(1);
    expect(xs.get(2)).toBe(1);

    xs.setOne(0);
    expect(xs.get(0)).toBe(1);
    expect(xs.get(1)).toBe(1);
    expect(xs.get(2)).toBe(1);
  });

  it('should throw errors if the index is out of bounds.', () => {
    expect(() => xs.get(-2)).toThrow();
    expect(() => xs.get(-1)).toThrow();
    expect(() => xs.get(3)).toThrow();
    expect(() => xs.get(4)).toThrow();
  });
});
