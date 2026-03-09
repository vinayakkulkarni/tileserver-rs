/**
 * Upload File Mutation
 *
 * Wraps $fetch POST /api/upload with TanStack useMutation.
 * Streams file to the Rust backend for server-side processing
 * (MBTiles, SQLite, COG formats).
 */

import { useMutation, useQueryClient } from '@tanstack/vue-query';
import type { UploadResponse } from '~/types/file-upload';
import { DATA_SOURCES_QUERY_KEYS } from '~/utils/query-keys';

export function useUploadFileMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (file: File): Promise<UploadResponse> => {
      const formData = new FormData();
      formData.append('file', file);

      return $fetch<UploadResponse>('/api/upload', {
        method: 'POST',
        body: formData,
      });
    },
    onSuccess: () => {
      // Invalidate data sources cache since a new source was added
      queryClient.invalidateQueries({ queryKey: DATA_SOURCES_QUERY_KEYS.all });
    },
  });
}
