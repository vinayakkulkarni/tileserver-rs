/**
 * useLlmPanel Composable
 *
 * Manages the LLM chat panel state: open/close, input, auto-scroll,
 * suggested prompts, and message helpers. Keeps Vue components thin.
 *
 * @see https://tanstack.com/ai/latest
 */

import { Map, Layers, Search, Globe } from '@lucide/vue';
import type { Map as MaplibreMap } from 'maplibre-gl';
import type { OverlayLayer } from '~/types/file-upload';
import type { MessagePart } from '@tanstack/ai';
import type { LlmIconName, SuggestedPrompt } from '~/types/llm';

const ICON_MAP: Record<LlmIconName, string> = {
  map: 'Map',
  layers: 'Layers',
  search: 'Search',
  palette: 'Palette',
  globe: 'Globe',
};

const ICON_COMPONENTS = { Map, Layers, Search, Globe } as const;

function getIconComponent(icon: string) {
  const key = icon.charAt(0).toUpperCase() + icon.slice(1);
  return ICON_COMPONENTS[key as keyof typeof ICON_COMPONENTS] ?? Map;
}

/**
 * Suggested prompts for new users
 */
const SUGGESTIONS: SuggestedPrompt[] = [
  {
    title: 'Fly to a city',
    prompt: 'Fly to Paris, France and show me the Eiffel Tower area',
    icon: 'map',
  },
  {
    title: 'Explore layers',
    prompt:
      'What layers are available on this map? Show me the current map state.',
    icon: 'layers',
  },
  {
    title: 'Find a place',
    prompt:
      'Take me to Tokyo, Japan at a good zoom level to see the city center',
    icon: 'search',
  },
  {
    title: 'View a region',
    prompt: 'Show me the entire Mediterranean Sea region',
    icon: 'globe',
  },
];

/**
 * Extract text content from TanStack AI message parts.
 * Accepts readonly array to support DeepReadonly messages from useChat.
 * For tool-capable models, text is clean. For fallback models,
 * strips [MAP_ACTION] blocks from the display text.
 */
export function getTextContent(parts: readonly MessagePart[]): string {
  const raw = parts
    .filter((part) => part.type === 'text')
    .map((part) => (part as { type: 'text'; content: string }).content)
    .join('');
  // Strip [MAP_ACTION] blocks from non-tool model output
  // For models with native tool calling, these blocks won't exist
  return raw
    .replace(/\[MAP_ACTION\]\{[\s\S]*?\}\[\/MAP_ACTION\]/g, '')
    .replace(/\[MAP_ACTION\][\s\S]*$/g, '')
    .trim();
}

/**
 * Format timestamp to readable time string
 */
export function formatMessageTime(date: Date | undefined): string {
  if (!date) return '';
  return new Intl.DateTimeFormat('en-US', {
    hour: 'numeric',
    minute: '2-digit',
    hour12: true,
  }).format(date);
}

/**
 * Composable for managing the LLM chat panel state and interactions.
 *
 * @param mapRef - Ref to the MapLibre GL map instance (for tool calling)
 * @param overlaysRef - Ref to overlay layers from file drops (for get_overlays tool)
 *
 * @example
 * ```ts
 * const {
 *   panelOpen, messages, input, isLoading, engineStatus,
 *   handleSubmit, handlePromptSelect, togglePanel,
 * } = useLlmPanel(mapRef, overlaysRef);
 * ```
 */
export function useLlmPanel(
  mapRef: Ref<MaplibreMap | null>,
  overlaysRef: Ref<readonly OverlayLayer[]> = ref<readonly OverlayLayer[]>([]),
) {
  const {
    status: engineStatus,
    loadProgress,
    errorMessage,
    selectedModel,
    availableModels,
    initEngine,
    resetEngine,
  } = useLlmEngine();
  const chat = useLlmChat(mapRef, overlaysRef);

  // Panel open/close state
  const panelOpen = ref(false);

  // Local input state
  const input = ref('');

  // Scroll anchor element at the bottom of message list
  const scrollAnchor = ref<HTMLElement | null>(null);

  /**
   * Parse WebLLM progress text into user-friendly stage descriptions
   */
  const loadStageText = computed(() => {
    const text = loadProgress.value.text.toLowerCase();
    if (text.includes('fetching') || text.includes('download'))
      return 'Downloading model files\u2026';
    if (text.includes('loading')) return 'Loading into memory\u2026';
    if (text.includes('compiling') || text.includes('shader'))
      return 'Compiling for your GPU\u2026';
    if (text === 'ready') return 'Ready!';
    return 'Initializing\u2026';
  });
  /**
   * Show loading indicator when assistant message has no text yet
   */
  const showLoadingIndicator = computed(() => {
    if (!chat.isLoading.value) return false;
    if (chat.messages.value.length === 0) return false;

    const lastMessage = chat.messages.value[chat.messages.value.length - 1];
    if (!lastMessage || lastMessage.role !== 'assistant') return false;

    const hasText = lastMessage.parts.some(
      (part) =>
        part.type === 'text' &&
        (part as { type: 'text'; content: string }).content.trim(),
    );
    return !hasText;
  });

  /**
   * Auto-scroll to bottom when new messages arrive
   */
  function scrollToBottom() {
    nextTick(() => {
      scrollAnchor.value?.scrollIntoView({ behavior: 'smooth', block: 'end' });
    });
  }

  watch(() => chat.messages.value, scrollToBottom, { deep: true });

  /**
   * Handle form submission
   */
  async function handleSubmit(e?: Event) {
    e?.preventDefault();
    const trimmedInput = input.value.trim();
    if (!trimmedInput || chat.isLoading.value) return;

    // Auto-initialize engine on first message
    if (engineStatus.value === 'idle') {
      await initEngine();
    }

    if (engineStatus.value !== 'ready') return;

    try {
      input.value = '';
      scrollToBottom();
      await chat.sendMessage(trimmedInput);
      scrollToBottom();
    } catch (error) {
      console.error('[LLM Panel] Error sending message:', error);
    }
  }
  /**
   * Handle suggested prompt selection
   */
  async function handlePromptSelect(prompt: string) {
    if (chat.isLoading.value) return;

    // Auto-initialize engine on first prompt
    if (engineStatus.value === 'idle') {
      await initEngine();
    }

    if (engineStatus.value !== 'ready') return;

    try {
      await chat.sendMessage(prompt);
    } catch (error) {
      console.error('[LLM Panel] Error sending prompt:', error);
    }
  }

  /**
   * Toggle panel open/close
   */
  function togglePanel() {
    panelOpen.value = !panelOpen.value;
  }

  /**
   * Switch to a different model. Resets the current engine and loads the new model.
   */
  async function selectModel(modelId: string) {
    const model = availableModels.find((m) => m.id === modelId);
    if (!model || model.id === selectedModel.value.id) return;
    await resetEngine();
    await initEngine(model);
  }

  return {
    // Panel state
    panelOpen,
    togglePanel,

    // Chat state
    messages: chat.messages,
    isLoading: chat.isLoading,
    error: chat.error,
    showLoadingIndicator,

    // Engine state
    engineStatus,
    loadProgress,
    loadStageText,
    selectedModel,
    availableModels,
    engineError: errorMessage,
    initEngine,
    selectModel,

    // Input
    input,

    // Chat methods
    stop: chat.stop,
    reload: chat.reload,
    clear: chat.clear,

    // Panel-specific
    scrollAnchor,
    handleSubmit,
    handlePromptSelect,

    // Static data
    suggestions: SUGGESTIONS,
    iconMap: ICON_MAP,

    // Helpers
    getTextContent,
    formatMessageTime,
    getIconComponent,
  };
}
