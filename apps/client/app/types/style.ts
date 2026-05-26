export interface Style {
  id: string;
  name: string;
  url: string;
  version: number;
  minzoom?: number;
  maxzoom?: number;
}

export interface TileJSON {
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
}
