/**
 * useLlmProviderSettings Composable (issue #1547)
 *
 * Form state and handlers for the Map AI provider settings view. Keeps
 * `LlmProviderSettings.vue` a thin template.
 */

import type { AcceptableValue } from 'reka-ui';
import { DEFAULT_BASE_URLS } from '~/lib/llm-remote';
import { useTestLlmConnectionMutation } from '~/utils/api/llm';
import type { LlmProvider } from '~/types/llm';
import { useLlmProvider } from './use-llm-provider';

function inputValue(e: Event): string {
  return e.target instanceof HTMLInputElement ? e.target.value : '';
}

export function useLlmProviderSettings() {
  const provider = useLlmProvider();
  const {
    activeProvider,
    activeSettings,
    baseUrlValid,
    remoteTarget,
    apiKey,
    rememberKey,
    selectProvider,
    updateSettings,
    setRememberKey,
    forgetKey,
    closeSettings,
  } = provider;

  const testMutation = useTestLlmConnectionMutation();

  /** Bring-your-own-key provider selected (needs base URL / key / model). */
  const isDirect = computed(
    () =>
      activeProvider.value === 'openai-compatible' ||
      activeProvider.value === 'anthropic',
  );

  const baseUrlPlaceholder = computed(() =>
    activeProvider.value === 'anthropic'
      ? DEFAULT_BASE_URLS.anthropic
      : DEFAULT_BASE_URLS['openai-compatible'],
  );

  const keyRequired = computed(() => activeProvider.value === 'anthropic');

  /** First missing setup item, or `null` when the provider is ready. */
  const setupHint = computed<string | null>(() => {
    if (!isDirect.value) return null;
    if (!baseUrlValid.value)
      return 'Base URL must use https:// (http:// only for localhost).';
    if (!activeSettings.value?.model.trim()) return 'Enter a model name.';
    if (keyRequired.value && !apiKey.value.trim()) return 'Enter your API key.';
    return null;
  });

  const testResult = computed(() => testMutation.data.value ?? null);
  const testing = computed(() => testMutation.isPending.value);

  function onProviderChange(value: AcceptableValue) {
    if (typeof value === 'string') {
      selectProvider(value as LlmProvider);
      testMutation.reset();
    }
  }

  function onBaseUrlInput(e: Event) {
    updateSettings({ baseUrl: inputValue(e) });
  }

  function onModelInput(e: Event) {
    updateSettings({ model: inputValue(e) });
  }

  function onKeyInput(e: Event) {
    apiKey.value = inputValue(e);
  }

  function onRememberChange(e: Event) {
    setRememberKey(e.target instanceof HTMLInputElement && e.target.checked);
  }

  function runTest() {
    const target = remoteTarget.value;
    if (target) testMutation.mutate(target);
  }

  function onForgetKey() {
    forgetKey();
    testMutation.reset();
  }

  return {
    ...provider,
    isDirect,
    baseUrlPlaceholder,
    keyRequired,
    setupHint,
    testResult,
    testing,
    rememberKey,
    onProviderChange,
    onBaseUrlInput,
    onModelInput,
    onKeyInput,
    onRememberChange,
    runTest,
    onForgetKey,
    closeSettings,
  };
}
