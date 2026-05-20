import { useMutation, useQueryClient } from '@tanstack/vue-query';
import { SERVER_QUERY_KEYS } from '~/utils/query-keys';

interface ReloadConfigResponse {
  status: string;
  config_hash?: string;
  sources?: number;
  styles?: number;
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
