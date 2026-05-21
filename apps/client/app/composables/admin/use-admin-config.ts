import { useQuery } from '@tanstack/vue-query';
import type {
  AdminBreadcrumbCrumb,
  ConfigLine,
  ConfigSectionSchema,
  ConfigSectionView,
} from '~/types';
import { adminConfigQueryOptions } from '~/utils/api/admin-config';
import { friendlyAdminError } from '~/utils/api/admin-mcp/friendly-error';
import { CONFIG_SCHEMA } from './config-schema';

const BREADCRUMBS: AdminBreadcrumbCrumb[] = [
  { label: 'Home', to: '/' },
  { label: 'Admin', to: '/admin' },
  { label: 'Configuration' },
];

const LINE_RE = /^([a-z_][\w.-]*)\s*=/i;

function parseLoadedToml(toml: string): Map<string, string[]> {
  const sections = new Map<string, string[]>();
  let currentHeader = '';
  for (const raw of toml.split('\n')) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    if (line.startsWith('[') && line.endsWith(']')) {
      currentHeader = line;
      if (!sections.has(currentHeader)) sections.set(currentHeader, []);
      continue;
    }
    if (currentHeader === '') continue;
    const bucket = sections.get(currentHeader) ?? [];
    bucket.push(raw);
    sections.set(currentHeader, bucket);
  }
  return sections;
}

function countSectionOccurrences(toml: string, header: string): number {
  if (!header.startsWith('[[')) {
    return toml.includes(`${header}\n`) || toml.endsWith(header) ? 1 : 0;
  }
  let count = 0;
  for (const raw of toml.split('\n')) {
    if (raw.trim() === header) count += 1;
  }
  return count;
}

function buildLinesForSection(
  section: ConfigSectionSchema,
  toml: string,
): { lines: ConfigLine[]; occurrences: number; isPresent: boolean } {
  const loaded = parseLoadedToml(toml);
  const occurrences = countSectionOccurrences(toml, section.header);
  const isPresent = occurrences > 0;

  const setBody = loaded.get(section.header) ?? [];
  const setKeyNames = new Set<string>();
  const setLines: ConfigLine[] = [];

  for (const raw of setBody) {
    const trimmed = raw.trim();
    const match = LINE_RE.exec(trimmed);
    if (!match) continue;
    const key = match[1] ?? '';
    if (!key) continue;
    setKeyNames.add(key);
    const fieldSchema = section.fields.find((f) => f.key === key) ?? null;
    setLines.push({ kind: 'set', key, rendered: raw, schema: fieldSchema });
  }

  const suggestionLines: ConfigLine[] = [];
  for (const field of section.fields) {
    if (setKeyNames.has(field.key)) continue;
    suggestionLines.push({ kind: 'suggestion', key: field.key, schema: field });
  }

  const lines: ConfigLine[] = [
    {
      kind: 'section-header',
      header: section.header,
      blurb: section.blurb,
      featureGate: section.featureGate,
    },
    ...setLines,
    ...suggestionLines,
  ];

  return { lines, occurrences, isPresent };
}

export function useAdminConfig() {
  const configQuery = useQuery(adminConfigQueryOptions());

  const payload = computed(() => configQuery.data.value ?? null);
  const isPending = computed(() => configQuery.isPending.value);
  const error = computed(() => configQuery.error.value);
  const friendly = computed(() => friendlyAdminError(error.value));

  const loadedToml = computed(() => payload.value?.toml ?? '');
  const sourcePath = computed(() => payload.value?.source_path ?? null);
  const configHashShort = computed(() => {
    const hash = payload.value?.config_hash;
    return hash ? hash.slice(0, 12) : '—';
  });

  const sections = computed<readonly ConfigSectionView[]>(() => {
    const toml = loadedToml.value;
    return CONFIG_SCHEMA.map((section) => {
      const { lines, occurrences, isPresent } = buildLinesForSection(section, toml);
      return { schema: section, lines, occurrences, isPresent };
    });
  });

  return {
    isPending,
    error,
    friendly,
    payload,
    loadedToml,
    sourcePath,
    configHashShort,
    sections,
    breadcrumbs: BREADCRUMBS,
  };
}
