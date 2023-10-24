import { describe, expect, it, test } from 'vitest';
import * as bits from './bits.js';
import './debug.js';
import { WaveletMatrix } from './waveletmatrix.js';

describe('WaveletMatrix', () => {
  // todo: add a 1 to punt to the large construction algorithm
  const wm = new WaveletMatrix([0, 1, 2, 1, 0, 1, 4, 1]);
  
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

  it('select', () => {
    expect(wm.select(0)).toBe(0);
    expect(wm.select(0, { k: 0 })).toBe(0);
    expect(wm.select(0, { k: 1 })).toBe(4);
    expect(wm.select(0, { k: 2 })).toBe(null);

    expect(wm.select(1)).toBe(1);
    expect(wm.select(1, { k: 0 })).toBe(1);
    expect(wm.select(1, { k: 1 })).toBe(3);
    expect(wm.select(1, { k: 2 })).toBe(5);
    expect(wm.select(1, { k: 3 })).toBe(7);
    expect(wm.select(1, { k: 4 })).toBe(null);

    expect(wm.select(2)).toBe(2);
    expect(wm.select(2, { k: 1 })).toBe(null);

    expect(wm.select(3)).toBe(null);
    expect(wm.select(3, { k: 1 })).toBe(null);

    expect(wm.select(4)).toBe(6);
    expect(wm.select(4, { k: 1 })).toBe(null);

    // test k parameter

    // test range parameter
  });

  // todo: test selectFromEnd in terms of select (use count to get the count, assert that the two are appropriately symmetric)

});
