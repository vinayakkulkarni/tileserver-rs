import { describe, expect, it } from 'vitest';
import { parseMapHash, resolveInitialView } from './initial-view';

const terrainStyle = {
  center: [86.925, 27.985],
  zoom: 12.5,
  bearing: -25,
  pitch: 65,
};

describe('resolveInitialView', () => {
  it('1. uses the URL hash, including bearing and pitch above 60', () => {
    expect(resolveInitialView('#10.5/34.1/77.62/-30/70', terrainStyle)).toEqual(
      { center: [77.62, 34.1], zoom: 10.5, bearing: -30, pitch: 70 },
    );
  });

  it('2. without a hash, uses the style camera', () => {
    expect(resolveInitialView('', terrainStyle)).toEqual({
      center: [86.925, 27.985],
      zoom: 12.5,
      bearing: -25,
      pitch: 65,
    });
  });

  it('3. without a hash or style camera, uses the world view', () => {
    expect(resolveInitialView('', { version: 8 } as object)).toEqual({
      center: [0, 0],
      zoom: 1,
      bearing: 0,
      pitch: 0,
    });
    expect(resolveInitialView('', null)).toEqual({
      center: [0, 0],
      zoom: 1,
      bearing: 0,
      pitch: 0,
    });
  });

  it('falls back to the style when the hash is invalid', () => {
    expect(resolveInitialView('#abc/1/2', terrainStyle).zoom).toBe(12.5);
    expect(resolveInitialView('#10/95/10', terrainStyle).zoom).toBe(12.5);
  });

  it('fills style zoom/bearing/pitch defaults when only center is set', () => {
    expect(resolveInitialView('', { center: [10, 20] })).toEqual({
      center: [10, 20],
      zoom: 1,
      bearing: 0,
      pitch: 0,
    });
  });
});

describe('parseMapHash', () => {
  it('accepts zoom/lat/lng with bearing and pitch optional', () => {
    expect(parseMapHash('#4.3/22/80')).toEqual({
      center: [80, 22],
      zoom: 4.3,
      bearing: 0,
      pitch: 0,
    });
  });

  it('rejects empty, short, out-of-range and over-pitched hashes', () => {
    expect(parseMapHash('')).toBeNull();
    expect(parseMapHash('#')).toBeNull();
    expect(parseMapHash('#5/10')).toBeNull();
    expect(parseMapHash('#30/10/10')).toBeNull();
    expect(parseMapHash('#5/10/10/200')).toBeNull();
    expect(parseMapHash('#5/10/10/0/86')).toBeNull();
    expect(parseMapHash('#5//10')).toBeNull();
  });
});
