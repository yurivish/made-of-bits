import { describe, expect, it, test } from 'vitest';
import * as bits from './bits.js';
import './debug.js';
import { WaveletMatrix } from './waveletmatrix.js';

describe('WaveletMatrix', () => {
  // todo: add a 1 to punt to the large construction algorithm
  const symbols = [0, 1, 2, 1, 0, 1, 4, 1];
  const wm = new WaveletMatrix(symbols);

  it('precedingCount', () => {
    expect(wm.precedingCount(0)).toBe(0);
    expect(wm.precedingCount(1)).toBe(2);
    expect(wm.precedingCount(2)).toBe(6);
    expect(wm.precedingCount(3)).toBe(7);
    expect(wm.precedingCount(4)).toBe(7);

    // test range parameter
    expect(wm.precedingCount(2, { range: { start: 0, end: 4 } })).toBe(3);
    expect(wm.precedingCount(2, { range: { start: 1, end: 4 } })).toBe(2);
    expect(wm.precedingCount(2, { range: { start: 1, end: wm.length } })).toBe(5);
    expect(wm.precedingCount(2, { range: { start: wm.length, end: wm.length } })).toBe(0);
    expect(wm.precedingCount(2, { range: { start: wm.length - 1, end: wm.length } })).toBe(1);
  });
  
  it('count', () => {
    expect(wm.count(0)).toBe(2);
    expect(wm.count(1)).toBe(4);
    expect(wm.count(2)).toBe(1);
    expect(wm.count(3)).toBe(0);
    expect(wm.count(4)).toBe(1);

    // test range parameter
    expect(wm.count(0, { range: { start: 0, end: 4 } })).toBe(1);
    expect(wm.count(0, { range: { start: 1, end: 4 } })).toBe(0);
    expect(wm.count(0, { range: { start: 1, end: wm.length } })).toBe(1);
    expect(wm.count(0, { range: { start: wm.length, end: wm.length } })).toBe(0);
  });

  it('quantile', () => {
    // [0, 1, 2, 1, 0, 1, 4, 1] sorted is
    // [0, 0, 1, 1, 1, 1, 2, 4]
    expect(wm.quantile(0)).toMatchObject({ symbol: 0, count: 2 });
    expect(wm.quantile(1)).toMatchObject({ symbol: 0, count: 2 });
    expect(wm.quantile(2)).toMatchObject({ symbol: 1, count: 4 });
    expect(wm.quantile(3)).toMatchObject({ symbol: 1, count: 4 });
    expect(wm.quantile(4)).toMatchObject({ symbol: 1, count: 4 });
    expect(wm.quantile(5)).toMatchObject({ symbol: 1, count: 4 });
    expect(wm.quantile(6)).toMatchObject({ symbol: 2, count: 1 });
    expect(wm.quantile(7)).toMatchObject({ symbol: 4, count: 1 });
    expect(() => wm.quantile(8)).toThrow();

    const options = { range: { start: 3, end: 6 } };
    expect(wm.quantile(0, options)).toMatchObject({ symbol: 0, count: 1 });
    expect(wm.quantile(1, options)).toMatchObject({ symbol: 1, count: 2 });
    expect(wm.quantile(2, options)).toMatchObject({ symbol: 1, count: 2 });
  });

  it('select', () => {
    // test select behavior and the k and range parameters
    expect(wm.select(0)).toBe(0);
    expect(wm.select(0, { k: 0 })).toBe(0);
    expect(wm.select(0, { k: 1 })).toBe(4);
    expect(wm.select(0, { k: 2 })).toBe(null);

    expect(wm.select(0, { k: 0, range: { start: 1, end: wm.length } })).toBe(4);
    expect(wm.select(0, { k: 1, range: { start: 1, end: wm.length } })).toBe(null);
    expect(wm.select(0, { k: 0, range: { start: 2, end: wm.length - 2 } })).toBe(4);
    expect(wm.select(0, { k: 1, range: { start: 2, end: wm.length - 2 } })).toBe(null);

    expect(wm.select(1)).toBe(1);
    expect(wm.select(1, { k: 0 })).toBe(1);
    expect(wm.select(1, { k: 1 })).toBe(3);
    expect(wm.select(1, { k: 2 })).toBe(5);
    expect(wm.select(1, { k: 3 })).toBe(7);
    expect(wm.select(1, { k: 4 })).toBe(null);

    expect(wm.select(1, { k: 0, range: { start: 2, end: wm.length - 2 } })).toBe(3);
    expect(wm.select(1, { k: 1, range: { start: 2, end: wm.length - 2 } })).toBe(5);
    expect(wm.select(1, { k: 2, range: { start: 2, end: wm.length - 2 } })).toBe(null);
    
    expect(wm.select(2)).toBe(2);
    expect(wm.select(2, { k: 1 })).toBe(null);

    expect(wm.select(2, { k: 0, range: { start: 2, end: wm.length - 2 } })).toBe(2);
    expect(wm.select(2, { k: 0, range: { start: 3, end: wm.length - 2 } })).toBe(null);

    expect(wm.select(3)).toBe(null);
    expect(wm.select(3, { k: 1 })).toBe(null);

    expect(wm.select(3, { k: 0, range: { start: 3, end: wm.length - 2 } })).toBe(null);
    expect(wm.select(3, { k: 1, range: { start: 3, end: wm.length - 2 } })).toBe(null);

    expect(wm.select(4)).toBe(6);
    expect(wm.select(4, { k: 1 })).toBe(null);

  });

  // Can we do generative testing in terms of select with arbitrary ranges?
  it('selectFromEnd', () => {
    symbols.forEach((symbol, i) => {
      expect(wm.selectFromEnd(symbol, { 
        // inefficient (O(symbols.length^2)) but with a short array this is fine
        k: symbols.slice(i + 1).filter(s => s === symbol).length 
      })).toBe(i);
    });

    // test range parameter
    expect(wm.selectFromEnd(0, { 
      k: 0, 
      range: { start: 1, end: wm.length } 
    })).toBe(4);
  });

  it('get', () => {
    expect(wm.get(0)).toBe(0);
    expect(wm.get(1)).toBe(1);
    expect(wm.get(2)).toBe(2);
    expect(wm.get(3)).toBe(1);
    expect(wm.get(4)).toBe(0);
    expect(wm.get(5)).toBe(1);
    expect(wm.get(6)).toBe(4);
    expect(wm.get(7)).toBe(1);
  });

  it('simpleMajority', () => {
    expect(wm.simpleMajority({ start: 0, end: wm.length })).toBe(null);
    expect(wm.simpleMajority({ start: 0, end: wm.length - 1 })).toBe(null);
    expect(wm.simpleMajority({ start: 1, end: wm.length })).toMatchObject({ symbol: 1, count: 4 });
    expect(wm.simpleMajority({ start: 3, end: wm.length })).toMatchObject({ symbol: 1, count: 3 });
    expect(wm.simpleMajority({ start: 2, end: 3 })).toMatchObject({ symbol: 2, count: 1 });
  });

  // console.log(wm.counts());
  // expect(wm.counts()).toMatchObject({});
});
