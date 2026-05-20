import { useMutation, useQueryClient } from '@tanstack/vue-query';
import { SERVER_QUERY_KEYS } from '~/utils/query-keys';

export interface ReloadConfigResponse {
  ok: boolean;
  reloaded: boolean;
  config_hash: string;
  loaded_at_unix: number;
  loaded_sources: number;
  loaded_styles: number;
  renderer_enabled: boolean;
  prometheus_listener_active: boolean;
  version: string;
}

export function useReloadConfigMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (): Promise<ReloadConfigResponse> => {
      return $fetch<ReloadConfigResponse>('/__admin/reload', {
        method: 'POST',
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SERVER_QUERY_KEYS.ping() });
    },
  });
}
