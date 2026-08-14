/**
 * Chat Database — TanStack DB Collections
 *
 * Persistent collection for chat messages.
 * Uses localStorage for cross-session persistence.
 *
 * NOTE: @tanstack/vue-db and @tanstack/db are listed in
 * vite.ssr.external (nuxt.config.ts) to avoid Vite SSR stack overflow
 * caused by circular re-exports in @tanstack/db internals.
 *
 * @see https://tanstack.com/db/latest
 */

import { z } from 'zod';
import {
  createCollection,
  localStorageCollectionOptions,
} from '@tanstack/vue-db';
import type { ChatMessage } from '~/types/llm';

// =============================================================================
// SCHEMAS (zod — used for runtime validation by TanStack DB)
// =============================================================================

const StoredToolCallSchema = z.object({
  id: z.string(),
  name: z.string(),
  args: z.string(),
  result: z.string().optional(),
});

const ChatMessageSchema = z.object({
  id: z.string(),
  role: z.enum(['user', 'assistant']),
  content: z.string(),
  timestamp: z.number(),
  toolCalls: z.array(StoredToolCallSchema).optional(),
});

// =============================================================================
// COLLECTIONS (externalized from SSR via vite.ssr.external)
// =============================================================================

export const chatCollection = createCollection(
  localStorageCollectionOptions({
    id: 'chat-messages',
    storageKey: 'tileserver-chat-messages',
    getKey: (msg: ChatMessage) => msg.id,
    schema: ChatMessageSchema,
  }),
);

