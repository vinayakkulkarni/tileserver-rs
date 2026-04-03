/**
 * LLM Chat Types - Frontend-specific
 *
 * Types for browser-local LLM integration via WebLLM + TanStack AI Vue.
 * @see https://tanstack.com/ai/latest
 * @see https://webllm.mlc.ai/
 */

// Re-export core types from TanStack AI Vue
import type {
  UIMessage as TanStackUIMessage,
  UseChatReturn,
} from '@tanstack/ai-vue';
import type { MessagePart as TanStackMessagePart } from '@tanstack/ai';

export type { UseChatReturn };

export type ChatWithEngineStatus = UseChatReturn & {
  engineStatus: LlmEngineStatus;
};

// Re-export message part types from TanStack AI
export type {
  TextPart,
  ToolCallPart,
  ToolResultPart,
  MessagePart,
} from '@tanstack/ai';

/**
 * Readonly UIMessage type - matches what TanStack AI Vue returns from useChat.
 * The messages from useChat are DeepReadonly, so we need a compatible type for components.
 */
export interface ReadonlyUIMessage {
  readonly id: string;
  readonly role: 'system' | 'user' | 'assistant';
  readonly parts: readonly TanStackMessagePart[];
  readonly createdAt?: Date;
}

// Re-export original UIMessage for cases where mutable is needed
export type { TanStackUIMessage as UIMessage };

// ============================================================================
// WEBLLM TYPES
// ============================================================================

/**
 * WebLLM model configuration
 */
export interface LlmModelConfig {
  /** Model identifier for WebLLM (e.g., 'Qwen3-4B-q4f16_1-MLC') */
  id: string;
  /** Human-readable display name */
  name: string;
  /** Approximate download size in GB */
  sizeGb: number;
  /** Whether this model supports WebLLM's native tool calling API */
  supportsTools: boolean;
}

/**
 * WebLLM engine loading progress
 */
export interface LlmLoadProgress {
  /** Progress text from WebLLM (e.g., 'Loading model...') */
  text: string;
  /** Progress value 0-1 */
  progress: number;
}

/**
 * WebLLM engine state
 */
export type LlmEngineStatus = 'idle' | 'loading' | 'ready' | 'error';

// ============================================================================
// MAP TOOLS
// ============================================================================

/**
 * Map tool call result from client tool execution
 */
export interface MapToolResult {
  toolCallId: string;
  toolName: string;
  result: unknown;
}

// ============================================================================
// CHAT PERSISTENCE TYPES (TanStack DB)
// ============================================================================

/**
 * Tool call record stored alongside a chat message
 */
export interface StoredToolCall {
  id: string;
  name: string;
  args: string;
  result?: string;
}

/**
 * Persisted chat message for TanStack DB collections.
 * Flattened from UIMessage parts for localStorage-friendly storage.
 */
export interface ChatMessage extends Record<string, unknown> {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
  toolCalls?: StoredToolCall[];
}

/**
 * Persisted spatial query result for TanStack DB collections.
 */
export interface SpatialResult extends Record<string, unknown> {
  id: string;
  /** Links to the ChatMessage.id that triggered this query */
  messageId: string;
  source: string;
  features: SpatialResultFeature[];
  total: number;
  truncated: boolean;
  timestamp: number;
}

/**
 * Individual feature in a spatial query result
 */
export interface SpatialResultFeature {
  layer: string;
  geometryType?: string;
  properties: Record<string, unknown>;
}

// ============================================================================
// OVERLAY TOOL TYPES
// ============================================================================

/**
 * Overlay info returned by the get_overlays tool.
 * Summarizes user-dropped file overlays visible on the map.
 */
export interface OverlayInfo {
  /** Unique overlay ID */
  id: string;
  /** Original file name (e.g., 'districts.geojson') */
  fileName: string;
  /** Detected format (geojson, kml, gpx, csv, shapefile, pmtiles) */
  format: string;
  /** Number of features in the overlay */
  featureCount: number;
  /** Assigned display color (hex) */
  color: string;
  /** Whether the overlay is currently visible */
  visible: boolean;
}

// ============================================================================
// PANEL TYPES
// ============================================================================

/**
 * Suggested prompts for the LLM chat (UI only)
 */
export interface SuggestedPrompt {
  title: string;
  prompt: string;
  icon: LlmIconName;
}

/**
 * Available icon names for suggested prompts
 */
export type LlmIconName = 'map' | 'layers' | 'search' | 'palette' | 'globe';

/**
 * Palette display mode:
 * - 'expanded'  — full chat panel, draggable
 * - 'minimized' — small floating pill showing status
 * - 'closed'    — completely hidden
 */
export type LlmPaletteMode = 'expanded' | 'minimized' | 'closed';

/**
 * Persisted position for the draggable palette
 */
export interface LlmPalettePosition {
  x: number;
  y: number;
}

// ============================================================================
// COMPONENT PROPS TYPES
// ============================================================================

/**
 * Props for LlmPanel component
 */
export interface LlmPanelProps {
  open: boolean;
}

/**
 * Emits for LlmPanel component
 */
export interface LlmPanelEmits {
  (e: 'update:open', value: boolean): void;
}

/**
 * Props for LlmMessageList component
 */
export interface LlmMessageListProps {
  messages: readonly ReadonlyUIMessage[];
  isLoading: boolean;
}

/**
 * Props for LlmInput component
 */
export interface LlmInputProps {
  modelValue: string;
  isLoading: boolean;
  engineStatus: LlmEngineStatus;
  hasMessages: boolean;
}

/**
 * Emits for LlmInput component
 */
export interface LlmInputEmits {
  (e: 'update:modelValue', value: string): void;
  (e: 'submit' | 'stop'): void;
}
