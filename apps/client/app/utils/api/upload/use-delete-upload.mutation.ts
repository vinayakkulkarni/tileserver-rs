/**
 * Delete Upload Mutation
 *
 * Wraps $fetch DELETE /api/upload/{id} with TanStack useMutation.
 * Removes the uploaded file and its registered tile source from the server.
 */

import { useMutation, useQueryClient } from '@tanstack/vue-query';
import { DATA_SOURCES_QUERY_KEYS } from '~/utils/query-keys';

export function useDeleteUploadMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (uploadId: string): Promise<void> => {
      await $fetch(`/api/upload/${uploadId}`, {
        method: 'DELETE',
      });
    },
    onSuccess: () => {
      // Invalidate data sources cache since a source was removed
      queryClient.invalidateQueries({ queryKey: DATA_SOURCES_QUERY_KEYS.all });
    },
  });
}
