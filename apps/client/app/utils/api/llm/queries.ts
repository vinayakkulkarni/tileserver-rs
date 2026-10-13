import type { LlmServerConfig } from '~/types/llm';
import { LLM_QUERY_KEYS } from '~/utils/query-keys';

async function fetchLlmConfig(): Promise<LlmServerConfig> {
  return await $fetch<LlmServerConfig>(apiUrl('/api/llm/config'));
}

/** `GET /api/llm/config` — proxy availability; never contains a key. */
export function llmConfigQueryOptions() {
  return {
    queryKey: LLM_QUERY_KEYS.config(),
    queryFn: fetchLlmConfig,
    staleTime: 5 * 60 * 1000,
  };
}
