export type Style = {
  id: string;
  name: string;
  url: string;
  version: number;
};

export type TileJSON = {
  tilejson: string;
  name: string;
  attribution: string;
  minzoom: number;
  maxzoom: number;
  bounds: number[];
  format: string;
  type: string;
  tiles: string[];
  center: number[];
};
