/**
 * useLlmProvider Composable (issue #1547)
 *
 * Which LLM backend the Map AI panel talks to, and its settings:
 * - `webllm` — local model in the browser (default)
 * - `openai-compatible` / `anthropic` — the user's own provider, called
 *   directly from the browser with their key (bring your own key)
 * - `proxy` — this server's `/api/llm/chat`, key held by the server
 *
 * The API key lives in `sessionStorage` (cleared when the tab closes) unless
 * the user opts in to "Remember key on this device" (`localStorage`). It is
 * only ever sent in request headers to the chosen provider.
 */

import { useLocalStorage, useSessionStorage } from '@vueuse/core';
import { useQuery } from '@tanstack/vue-query';
import { llmConfigQueryOptions } from '~/utils/api/llm';
import {
  DEFAULT_BASE_URLS,
  directTarget,
  hostOf,
  isAllowedBaseUrl,
  proxyTarget,
} from '~/lib/llm-remote';
import type {
  LlmProvider,
  LlmProviderOption,
  RemoteChatTarget,
  RemoteProviderKind,
  RemoteProviderSettings,
} from '~/types/llm';

const KEY_STORAGE = 'tileserver-llm-api-key';

const provider = useLocalStorage<LlmProvider>(
  'tileserver-llm-provider',
  'webllm',
);
const settings = useLocalStorage<
  Record<RemoteProviderKind, RemoteProviderSettings>
>(
  'tileserver-llm-remote-settings',
  {
    'openai-compatible': { baseUrl: '', model: '' },
    anthropic: { baseUrl: '', model: '' },
  },
  { mergeDefaults: true },
);
const rememberKey = useLocalStorage('tileserver-llm-remember-key', false);
const sessionKey = useSessionStorage(KEY_STORAGE, '');
const persistentKey = useLocalStorage(KEY_STORAGE, '');
/** Whether the provider settings view is shown inside the panel. */
const settingsOpen = ref(false);

/** Key for the active store; switching stores moves the key across. */
const apiKey = computed<string>({
  get: () => (rememberKey.value ? persistentKey.value : sessionKey.value),
  set: (value) => {
    if (rememberKey.value) persistentKey.value = value;
    else sessionKey.value = value;
  },
});

function setRememberKey(remember: boolean) {
  const current = apiKey.value;
  rememberKey.value = remember;
  persistentKey.value = remember ? current : '';
  sessionKey.value = remember ? '' : current;
}

function forgetKey() {
  persistentKey.value = '';
  sessionKey.value = '';
}

const PROVIDER_LABELS: Record<LlmProvider, string> = {
  webllm: 'Local (WebLLM, private)',
  'openai-compatible': 'OpenAI-compatible',
  anthropic: 'Anthropic (Claude)',
  proxy: 'Server provider',
};

export function useLlmProvider() {
  const serverConfigQuery = useQuery(llmConfigQueryOptions());
  const serverConfig = computed(() => serverConfigQuery.data.value ?? null);
  /** `/api/llm/config` has not answered yet. */
  const configPending = computed(() => serverConfigQuery.isPending.value);

  /** Wait until the server config is known (resolves on success or error). */
  async function whenConfigReady(): Promise<void> {
    if (!configPending.value) return;
    await serverConfigQuery.suspense().catch(() => undefined);
  }

  const userKeysAllowed = computed(
    () => serverConfig.value?.allow_user_keys ?? true,
  );
  const proxyAvailable = computed(
    () => serverConfig.value?.proxy_enabled ?? false,
  );

  const options = computed<LlmProviderOption[]>(() => {
    const values: LlmProvider[] = ['webllm'];
    if (proxyAvailable.value) values.push('proxy');
    if (userKeysAllowed.value) values.push('openai-compatible', 'anthropic');
    return values.map((value) => ({ value, label: PROVIDER_LABELS[value] }));
  });

  /**
   * Selected provider, falling back to WebLLM if the server disallows it.
   * While the server config is still loading, keep the stored choice so a
   * saved "Server provider" is not replaced by WebLLM on page load.
   */
  const activeProvider = computed<LlmProvider>(() => {
    if (configPending.value) return provider.value;
    return options.value.some((o) => o.value === provider.value)
      ? provider.value
      : 'webllm';
  });

  const isRemote = computed(() => activeProvider.value !== 'webllm');

  const remoteKind = computed<RemoteProviderKind | null>(() => {
    const p = activeProvider.value;
    if (p === 'webllm') return null;
    if (p === 'proxy')
      return serverConfig.value?.provider ?? 'openai-compatible';
    return p;
  });

  /** Settings of the selected bring-your-own-key provider. */
  const activeSettings = computed<RemoteProviderSettings | null>(() => {
    const p = activeProvider.value;
    return p === 'openai-compatible' || p === 'anthropic'
      ? settings.value[p]
      : null;
  });

  const effectiveBaseUrl = computed(() => {
    const p = activeProvider.value;
    if (p !== 'openai-compatible' && p !== 'anthropic') return '';
    return settings.value[p].baseUrl.trim() || DEFAULT_BASE_URLS[p];
  });

  const baseUrlValid = computed(() => isAllowedBaseUrl(effectiveBaseUrl.value));

  /** Ready-to-send request target, or `null` when setup is incomplete. */
  const remoteTarget = computed<RemoteChatTarget | null>(() => {
    const p = activeProvider.value;
    if (p === 'proxy') {
      const config = serverConfig.value;
      if (!config?.proxy_enabled) return null;
      return proxyTarget(config.provider, apiUrl('/api/llm/chat'));
    }
    if (p !== 'openai-compatible' && p !== 'anthropic') return null;
    const { model } = settings.value[p];
    if (!baseUrlValid.value || !model.trim()) return null;
    if (p === 'anthropic' && !apiKey.value.trim()) return null;
    return directTarget(p, effectiveBaseUrl.value, model, apiKey.value);
  });

  /** Host that receives prompts in remote mode (for the privacy notice). */
  const destinationHost = computed(() => {
    if (activeProvider.value === 'proxy') {
      return serverConfig.value?.upstream_host ?? 'this server';
    }
    return isRemote.value ? hostOf(effectiveBaseUrl.value) : null;
  });

  /** Short "model · host" label for the header in remote mode. */
  const remoteLabel = computed(() => {
    if (activeProvider.value === 'proxy') {
      return serverConfig.value?.model ?? 'Server model';
    }
    return activeSettings.value?.model.trim() || 'Not set up';
  });

  function selectProvider(value: LlmProvider) {
    provider.value = value;
  }

  function openSettings() {
    settingsOpen.value = true;
  }

  function closeSettings() {
    settingsOpen.value = false;
  }

  function toggleSettings() {
    settingsOpen.value = !settingsOpen.value;
  }

  function updateSettings(patch: Partial<RemoteProviderSettings>) {
    const p = activeProvider.value;
    if (p !== 'openai-compatible' && p !== 'anthropic') return;
    settings.value = {
      ...settings.value,
      [p]: { ...settings.value[p], ...patch },
    };
  }

  return {
    options,
    activeProvider,
    isRemote,
    remoteKind,
    activeSettings,
    effectiveBaseUrl,
    baseUrlValid,
    remoteTarget,
    destinationHost,
    remoteLabel,
    apiKey,
    rememberKey: readonly(rememberKey),
    serverConfig,
    configPending,
    whenConfigReady,
    settingsOpen: readonly(settingsOpen),
    openSettings,
    closeSettings,
    toggleSettings,
    selectProvider,
    updateSettings,
    setRememberKey,
    forgetKey,
  };
}
