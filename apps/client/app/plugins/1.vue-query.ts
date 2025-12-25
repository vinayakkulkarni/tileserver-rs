/**
 * TanStack Query Plugin
 *
 * Initializes the QueryClient which powers both:
 * 1. Traditional TanStack Query hooks (useQuery, useMutation)
 * 2. TanStack DB Collections (via queryCollectionOptions)
 *
 * The QueryClient is exported via `setQueryClient()` for use by collections.
 */

import type {
  DehydratedState,
  VueQueryPluginOptions,
} from '@tanstack/vue-query';
import {
  VueQueryPlugin,
  QueryClient,
  hydrate,
  dehydrate,
} from '@tanstack/vue-query';
import { setQueryClient } from '#shared/collections/query-client';

export default defineNuxtPlugin({
  name: 'vue-query',
  setup(nuxt) {
    const vueQueryState = useState<DehydratedState | null>('vue-query');

    const queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: 3,
          refetchOnWindowFocus: false,
          staleTime: 30 * 1000, // 30 seconds default
        },
      },
    });

    // Export QueryClient for TanStack DB collections
    setQueryClient(queryClient);

    // Client persister for maintaining state across navigation
    const clientPersister = import.meta.client
      ? (client: QueryClient): [() => void, Promise<void>] => [
          () => {
            vueQueryState.value = dehydrate(client);
          },
          Promise.resolve().then(() => {
            if (vueQueryState.value) {
              hydrate(client, vueQueryState.value);
            }
          }),
        ]
      : undefined;

    const options: VueQueryPluginOptions = {
      queryClient,
      ...(clientPersister && { clientPersister }),
    };

    nuxt.vueApp.use(VueQueryPlugin, options);

    // Server-side: dehydrate state after rendering
    if (import.meta.server) {
      nuxt.hooks.hook('app:rendered', () => {
        vueQueryState.value = dehydrate(queryClient);
      });
    }

    // Client-side: hydrate state on creation
    if (import.meta.client) {
      nuxt.hooks.hook('app:created', () => {
        if (vueQueryState.value) {
          hydrate(queryClient, vueQueryState.value);
        }
      });
    }

    return {
      provide: {
        queryClient,
      },
    };
  },
});
