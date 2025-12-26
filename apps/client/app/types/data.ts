export interface VectorLayer {
  id: string;
  fields: Record<string, string>;
  minzoom: number;
  maxzoom: number;
}

export interface Data {
  tilejson: string;
  tiles: string[];
  name: string;
  format: string;
  basename: string;
  id: string;
  type: string;
  version: string;
  description: string;
  minzoom: number;
  maxzoom: number;
  bounds: number[];
  center: number[];
  vector_layers: VectorLayer[];
}

export interface LayerColor {
  id: string;
  color: string;
}
