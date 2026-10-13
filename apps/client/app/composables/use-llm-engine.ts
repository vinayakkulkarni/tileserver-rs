/**
 * useLlmEngine Composable
 *
 * Manages the WebLLM engine lifecycle: initialization, model loading,
 * progress tracking, and cleanup. The engine runs entirely in-browser
 * via WebGPU — no server required.
 *
 * @see https://webllm.mlc.ai/
 */

import { CreateMLCEngine } from '@mlc-ai/web-llm';
import type { MLCEngine } from '@mlc-ai/web-llm';
import type {
  LlmEngineStatus,
  LlmLoadProgress,
  LlmModelConfig,
} from '~/types/llm';

/** Default model — the first entry of AVAILABLE_MODELS. */
const DEFAULT_MODEL: LlmModelConfig = {
  id: 'Qwen3.5-4B-q4f16_1-MLC',
  name: 'Qwen3.5 4B (recommended)',
  sizeGb: 3.9,
  supportsTools: false,
};

/**
 * Available models for the chat.
 *
 * WebLLM's native tool-calling API (`tools` param) is restricted to Hermes
 * models only via an internal allowlist. All Qwen3.5 models use our text-based
 * [MAP_ACTION] parser + parseToolIntentsFromText() fallback instead —
 * which works well because Qwen3.5 is a strong instruction follower.
 *
 * Sizes are `vram_required_MB` from `prebuiltAppConfig` in
 * `@mlc-ai/web-llm@0.2.86`.
 */
const AVAILABLE_MODELS: LlmModelConfig[] = [
  DEFAULT_MODEL,
  {
    id: 'Qwen3.5-9B-q4f16_1-MLC',
    name: 'Qwen3.5 9B (best quality)',
    sizeGb: 6.4,
    supportsTools: false,
  },
  {
    id: 'Qwen3.5-2B-q4f16_1-MLC',
    name: 'Qwen3.5 2B (lightweight)',
    sizeGb: 2.2,
    supportsTools: false,
  },
  {
    id: 'Qwen3.5-0.8B-q4f16_1-MLC',
    name: 'Qwen3.5 0.8B (minimal)',
    sizeGb: 1.6,
    supportsTools: false,
  },
];

/**
 * Shared engine state — singleton across the app.
 * The engine persists across component mounts so the model
 * isn't re-downloaded when toggling the chat panel.
 */
const engine = shallowRef<MLCEngine | null>(null);
const status = ref<LlmEngineStatus>('idle');
const loadProgress = ref<LlmLoadProgress>({ text: '', progress: 0 });
const errorMessage = ref<string | null>(null);
const selectedModel = ref<LlmModelConfig>(DEFAULT_MODEL);

/**
 * Composable for managing WebLLM engine lifecycle.
 *
 * @example
 * ```ts
 * const { engine, status, loadProgress, initEngine } = useLlmEngine();
 * await initEngine(); // Downloads + compiles model (~30s first time)
 * ```
 */
export function useLlmEngine() {
  /**
   * Check if WebGPU is available in this browser
   */
  function isWebGpuSupported(): boolean {
    return typeof navigator !== 'undefined' && 'gpu' in navigator;
  }

  /**
   * Initialize the WebLLM engine with the selected model.
   * Downloads and compiles the model on first run (~30s).
   * Subsequent loads use cached model from IndexedDB (~2-5s).
   */
  async function initEngine(modelConfig?: LlmModelConfig): Promise<void> {
    if (status.value === 'loading') return;
    if (engine.value && status.value === 'ready') return;

    if (!isWebGpuSupported()) {
      status.value = 'error';
      errorMessage.value =
        'WebGPU is not supported in this browser. Try Chrome 113+ or Edge 113+.';
      return;
    }

    const model = modelConfig ?? selectedModel.value;
    selectedModel.value = model;
    status.value = 'loading';
    errorMessage.value = null;
    loadProgress.value = { text: 'Initializing...', progress: 0 };

    try {
      const mlcEngine = await CreateMLCEngine(model.id, {
        initProgressCallback: (progress) => {
          loadProgress.value = {
            text: progress.text,
            progress: progress.progress,
          };
        },
      });

      engine.value = mlcEngine;
      status.value = 'ready';
      loadProgress.value = { text: 'Ready', progress: 1 };
    } catch (err) {
      status.value = 'error';
      errorMessage.value =
        err instanceof Error ? err.message : 'Failed to load model';
      console.error('[LLM Engine] Failed to initialize:', err);
    }
  }

  /**
   * Reset the engine (unload model, free memory)
   */
  async function resetEngine(): Promise<void> {
    if (engine.value) {
      try {
        await engine.value.resetChat();
      } catch {
        // Ignore reset errors
      }
      engine.value = null;
    }
    status.value = 'idle';
    loadProgress.value = { text: '', progress: 0 };
    errorMessage.value = null;
  }

  return {
    engine: engine as Readonly<ShallowRef<MLCEngine | null>>,
    status: readonly(status),
    loadProgress: readonly(loadProgress),
    errorMessage: readonly(errorMessage),
    selectedModel: readonly(selectedModel),
    availableModels: AVAILABLE_MODELS,
    isWebGpuSupported,
    initEngine,
    resetEngine,
  };
}
