/**
 * TanStack DB Plugin
 *
 * Collections are created using the function-based composable pattern
 * (e.g., useDataSourcesCollection, useMapStylesCollection).
 *
 * The function-based pattern creates collections inside Vue composables with
 * useQueryClient(), which ensures proper Vue reactivity integration.
 *
 * @see https://tanstack.com/db/latest
 */

export default defineNuxtPlugin({
  name: 'tanstack-db',
  dependsOn: ['vue-query'],
  setup() {
    // Only run on client
    if (!import.meta.client) {
      return;
    }

    // Dev logging
    if (import.meta.dev) {
      console.log(
        '[TanStack DB] Plugin loaded - collections use function-based composables',
      );
    }

    return {
      provide: {
        /**
         * Cleanup is handled by individual collections
         */
        tanstackDbCleanup: () => {
          if (import.meta.dev) {
            console.log(
              '[TanStack DB] tanstackDbCleanup called - no-op with function-based pattern',
            );
          }
        },

        /**
         * Re-init is handled by individual collections
         */
        tanstackDbReinit: () => {
          if (import.meta.dev) {
            console.log(
              '[TanStack DB] tanstackDbReinit called - no-op with function-based pattern',
            );
          }
        },
      },
    };
  },
});
