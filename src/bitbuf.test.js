import { describe, expect, it, test } from 'vitest';
import { BitBuf } from './bitbuf.js';

describe('BitBuf', () => {
  const xs = new BitBuf(3);

  it('should be initialized to false', () => {

    expect(xs.get(0)).toBe(false);
    expect(xs.get(1)).toBe(false);
    expect(xs.get(2)).toBe(false);
  });

  it('should set bits individually', () => {
    xs.set(1);
    expect(xs.get(0)).toBe(false);
    expect(xs.get(1)).toBe(true);
    expect(xs.get(2)).toBe(false);

    xs.set(2);
    expect(xs.get(0)).toBe(false);
    expect(xs.get(1)).toBe(true);
    expect(xs.get(2)).toBe(true);

    xs.set(0);
    expect(xs.get(0)).toBe(true);
    expect(xs.get(1)).toBe(true);
    expect(xs.get(2)).toBe(true);
  });

  it('should throw errors if the index is out of bounds.', () => {
    expect(() => xs.get(-2)).toThrow();
    expect(() => xs.get(-1)).toThrow();
    expect(() => xs.get(3)).toThrow();
    expect(() => xs.get(4)).toThrow();
  });
});
