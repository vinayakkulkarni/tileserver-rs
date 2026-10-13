import { useMutation } from '@tanstack/vue-query';
import { testConnection } from '~/lib/llm-remote';
import type { RemoteChatTarget } from '~/types/llm';

/** Send a tiny chat request to check that the provider, key and model work. */
export function useTestLlmConnectionMutation() {
  return useMutation({
    mutationFn: (target: RemoteChatTarget) => testConnection(target),
  });
}
