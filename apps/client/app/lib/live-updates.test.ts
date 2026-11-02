import { describe, expect, it } from 'vitest';
import {
  isDataUrlFor,
  liveSocketUrl,
  parseLiveMessage,
  reconnectDelayMs,
  styleSourcesFor,
} from './live-updates';

describe('liveSocketUrl', () => {
  it('maps http(s) to ws(s) and keeps the base path', () => {
    expect(liveSocketUrl('http://localhost:8080', '/ws')).toBe(
      'ws://localhost:8080/ws',
    );
    expect(liveSocketUrl('https://demo.tileserver.app', '/maps/ws')).toBe(
      'wss://demo.tileserver.app/maps/ws',
    );
  });
});

describe('parseLiveMessage', () => {
  it('accepts the two server messages', () => {
    expect(parseLiveMessage('{"type":"reload","source":"osm"}')).toEqual({
      type: 'reload',
      source: 'osm',
    });
    expect(parseLiveMessage('{"type":"hello","sources":["a","b"]}')).toEqual({
      type: 'hello',
      sources: ['a', 'b'],
    });
  });

  it('rejects anything else', () => {
    for (const text of [
      'not json',
      'null',
      '{"type":"reload"}',
      '{"type":"reload","source":1}',
      '{"type":"hello","sources":[1]}',
      '{"type":"other"}',
    ]) {
      expect(parseLiveMessage(text)).toBeNull();
    }
  });
});

describe('isDataUrlFor / styleSourcesFor', () => {
  it('matches TileJSON and tile URLs of one source only', () => {
    expect(isDataUrlFor('/data/osm.json', 'osm')).toBe(true);
    expect(isDataUrlFor('http://h/data/osm/1/2/3.pbf', 'osm')).toBe(true);
    expect(isDataUrlFor('/maps/data/osm', 'osm')).toBe(true);
    expect(isDataUrlFor('/data/osm-extra.json', 'osm')).toBe(false);
    expect(isDataUrlFor('/styles/osm/style.json', 'osm')).toBe(false);
  });

  it('finds every style source fed by the server source', () => {
    const sources = {
      base: { url: '/data/osm.json' },
      raster: { tiles: ['http://h/data/osm/{z}/{x}/{y}.png'] },
      other: { url: '/data/terrain.json' },
      inline: {},
    };
    expect(styleSourcesFor(sources, 'osm')).toEqual(['base', 'raster']);
    expect(styleSourcesFor(sources, 'none')).toEqual([]);
  });
});

describe('reconnectDelayMs', () => {
  it('backs off exponentially to 30 s', () => {
    expect([1, 2, 3, 6, 20].map(reconnectDelayMs)).toEqual([
      1000, 2000, 4000, 30_000, 30_000,
    ]);
  });
});
