export type Data = {
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
  tilejson: string;
};

type VectorLayer = {
  id: string;
  fields: Fields;
  minzoom: number;
  maxzoom: number;
};

type Fields = {
  class?: string;
  iso_a2?: string;
  'name:latin'?: string;
  pop?: string;
  rank?: string;
  admin_level?: string;
  disputed?: string;
  subclass?: string;
  housenumber?: string;
  brunnel?: string;
  intermittent?: string;
  oneway?: string;
  ramp?: string;
  service?: string;
  network?: string;
  ref?: string;
  ref_length?: string;
  render_height?: string;
  render_min_height?: string;
  ele?: string;
  ele_ft?: string;
  iata?: string;
  icao?: string;
};
