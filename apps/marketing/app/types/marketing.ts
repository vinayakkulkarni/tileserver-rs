import type { Component } from 'vue';

export type FeatureCategory =
  | 'Data formats'
  | 'Rendering'
  | 'Developer experience'
  | 'Deployment'
  | 'Intelligence';

export interface Feature {
  icon: Component;
  title: string;
  description: string;
  category: FeatureCategory;
}

export interface FeatureGroup {
  category: FeatureCategory;
  features: Feature[];
}

export type ComparisonCell = '✓' | '✗' | '◐';

export type ComparisonColumn =
  | 'tileserver-rs'
  | 'martin'
  | 'tileserver-gl'
  | 'pg_tileserv'
  | 'titiler';

export interface ComparisonRow {
  feature: string;
  values: Record<ComparisonColumn, ComparisonCell>;
}

export interface ApiEndpoint {
  method: string;
  path: string;
}

export interface ApiEndpointGroup {
  title: string;
  endpoints: ApiEndpoint[];
}

export interface PerformanceStat {
  icon: Component;
  value: number;
  label: string;
  detail: string;
  prefix?: string;
  /** Digits after the decimal point; defaults to 0. */
  decimals?: number;
}

export interface AiBenefit {
  icon: Component;
  title: string;
  description: string;
}

export interface AiChatMessage {
  role: 'user' | 'assistant';
  text: string;
}
