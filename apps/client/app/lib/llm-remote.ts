/**
 * Remote LLM providers for the Map AI panel (issue #1547).
 *
 * Pure helpers — no Vue, no Nuxt — so they can be unit-tested with fixtures:
 * - base URL validation and endpoint resolution
 * - OpenAI Chat Completions and Anthropic Messages request builders
 * - streaming (SSE) parsers that turn either wire format into
 *   provider-neutral {@link RemoteStreamEvent}s
 *
 * The browser talks to the provider directly (bring your own key) or to this
 * server's `/api/llm/chat` proxy, which holds the key server-side.
 */

import type { ModelMessage } from '@tanstack/ai';
import type {
  AnthropicTool,
  LlmConnectionTestResult,
  OpenAiTool,
  RemoteChatTarget,
  RemoteProviderKind,
  RemoteStreamEvent,
} from '~/types/llm';

/** `anthropic-version` header for the Messages API. */
const ANTHROPIC_VERSION = '2023-06-01';

/** Default base URL per provider family (shown as the input placeholder). */
export const DEFAULT_BASE_URLS: Record<RemoteProviderKind, string> = {
  'openai-compatible': 'https://api.openai.com/v1',
  anthropic: 'https://api.anthropic.com',
};

/** Max tokens per assistant turn (required by Anthropic, sent to both). */
const MAX_TOKENS = 1024;

const LOOPBACK_HOSTS = ['localhost', '127.0.0.1', '[::1]'];

/**
 * Whether `url` may be used as a provider base URL: `https://` for any host,
 * plain `http://` only for loopback (local Ollama / LM Studio / vLLM).
 * Mirrors `is_allowed_llm_base_url` on the server.
 */
export function isAllowedBaseUrl(url: string): boolean {
  let parsed: URL;
  try {
    parsed = new URL(url.trim());
  } catch {
    return false;
  }
  if (!parsed.hostname) return false;
  if (parsed.protocol === 'https:') return true;
  if (parsed.protocol !== 'http:') return false;
  return LOOPBACK_HOSTS.includes(parsed.hostname);
}

/** Chat endpoint for `kind` under `baseUrl` (trailing slashes ignored). */
export function endpointFor(kind: RemoteProviderKind, baseUrl: string): string {
  const base = baseUrl.trim().replace(/\/+$/, '');
  if (kind === 'openai-compatible') return `${base}/chat/completions`;
  return base.endsWith('/v1') ? `${base}/messages` : `${base}/v1/messages`;
}

/** Host (with port) of a URL, for the "prompts are sent to <host>" notice. */
export function hostOf(url: string): string {
  try {
    return new URL(url.trim()).host;
  } catch {
    return url;
  }
}

/**
 * Build the target for a direct, bring-your-own-key request.
 * The key is only placed in request headers — never in the URL.
 */
export function directTarget(
  kind: RemoteProviderKind,
  baseUrl: string,
  model: string,
  apiKey: string,
): RemoteChatTarget {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
  };
  const key = apiKey.trim();
  if (kind === 'anthropic') {
    headers['anthropic-version'] = ANTHROPIC_VERSION;
    headers['anthropic-dangerous-direct-browser-access'] = 'true';
    if (key) headers['x-api-key'] = key;
  } else if (key) {
    headers.authorization = `Bearer ${key}`;
  }
  return {
    kind,
    url: endpointFor(kind, baseUrl || DEFAULT_BASE_URLS[kind]),
    headers,
    model: model.trim(),
  };
}

/** Build the target for this server's `/api/llm/chat` proxy. */
export function proxyTarget(
  kind: RemoteProviderKind,
  proxyUrl: string,
): RemoteChatTarget {
  return {
    kind,
    url: proxyUrl,
    headers: { 'content-type': 'application/json' },
    model: null,
  };
}

/** Flatten message content to plain text (non-text parts are dropped). */
function contentToText(content: ModelMessage['content']): string {
  if (typeof content === 'string') return content;
  if (!content) return '';
  return content
    .map((part) => (part.type === 'text' ? part.content : ''))
    .join('');
}

type OpenAiMessage =
  | { role: 'system' | 'user'; content: string }
  | {
      role: 'assistant';
      content: string;
      tool_calls?: Array<{
        id: string;
        type: 'function';
        function: { name: string; arguments: string };
      }>;
    }
  | { role: 'tool'; content: string; tool_call_id: string };

/** Convert TanStack model messages to OpenAI Chat Completions messages. */
function toOpenAiMessages(
  messages: ModelMessage[],
  systemPrompt: string,
): OpenAiMessage[] {
  const result: OpenAiMessage[] = [{ role: 'system', content: systemPrompt }];
  for (const msg of messages) {
    const content = contentToText(msg.content);
    if (msg.role === 'user') {
      result.push({ role: 'user', content });
    } else if (msg.role === 'assistant') {
      const toolCalls = (msg.toolCalls ?? []).map((call) => ({
        id: call.id,
        type: 'function' as const,
        function: {
          name: call.function.name,
          arguments: call.function.arguments,
        },
      }));
      result.push(
        toolCalls.length > 0
          ? { role: 'assistant', content, tool_calls: toolCalls }
          : { role: 'assistant', content },
      );
    } else if (msg.toolCallId) {
      result.push({ role: 'tool', content, tool_call_id: msg.toolCallId });
    }
  }
  return result;
}

type AnthropicBlock =
  | { type: 'text'; text: string }
  | { type: 'tool_use'; id: string; name: string; input: unknown }
  | { type: 'tool_result'; tool_use_id: string; content: string };

interface AnthropicMessage {
  role: 'user' | 'assistant';
  content: AnthropicBlock[];
}

function parseToolInput(args: string): unknown {
  if (!args.trim()) return {};
  try {
    return JSON.parse(args);
  } catch {
    return {};
  }
}

/**
 * Convert TanStack model messages to Anthropic Messages format. Tool results
 * become `tool_result` blocks in a user turn; consecutive turns with the same
 * role are merged, as the API requires alternating roles.
 */
export function toAnthropicMessages(
  messages: ModelMessage[],
): AnthropicMessage[] {
  const result: AnthropicMessage[] = [];
  const push = (role: AnthropicMessage['role'], blocks: AnthropicBlock[]) => {
    if (blocks.length === 0) return;
    const last = result.at(-1);
    if (last && last.role === role) last.content.push(...blocks);
    else result.push({ role, content: blocks });
  };
  for (const msg of messages) {
    const text = contentToText(msg.content);
    if (msg.role === 'user') {
      push('user', text ? [{ type: 'text', text }] : []);
    } else if (msg.role === 'assistant') {
      const blocks: AnthropicBlock[] = text ? [{ type: 'text', text }] : [];
      for (const call of msg.toolCalls ?? []) {
        blocks.push({
          type: 'tool_use',
          id: call.id,
          name: call.function.name,
          input: parseToolInput(call.function.arguments),
        });
      }
      push('assistant', blocks);
    } else if (msg.toolCallId) {
      push('user', [
        { type: 'tool_result', tool_use_id: msg.toolCallId, content: text },
      ]);
    }
  }
  return result;
}

/** Convert OpenAI-style function tools to Anthropic tool definitions. */
export function toAnthropicTools(tools: OpenAiTool[]): AnthropicTool[] {
  return tools.map((tool) => ({
    name: tool.function.name,
    description: tool.function.description,
    input_schema: tool.function.parameters,
  }));
}

/** Build the JSON request body for one streaming chat turn. */
export function buildRequestBody(
  target: RemoteChatTarget,
  messages: ModelMessage[],
  systemPrompt: string,
  tools: OpenAiTool[],
): Record<string, unknown> {
  const body: Record<string, unknown> = {
    stream: true,
    max_tokens: MAX_TOKENS,
  };
  if (target.model) body.model = target.model;
  if (target.kind === 'anthropic') {
    body.system = systemPrompt;
    body.messages = toAnthropicMessages(messages);
    if (tools.length > 0) body.tools = toAnthropicTools(tools);
  } else {
    body.messages = toOpenAiMessages(messages, systemPrompt);
    if (tools.length > 0) body.tools = tools;
  }
  return body;
}

/**
 * Split a byte stream into SSE `data:` payloads. Handles events split across
 * chunks, CRLF line endings, multi-line data, and comment lines.
 */
export async function* readSseData(
  stream: ReadableStream<Uint8Array>,
): AsyncGenerator<string> {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  let data: string[] = [];
  try {
    while (true) {
      const { done, value } = await reader.read();
      buffer += done
        ? decoder.decode()
        : decoder.decode(value, { stream: true });
      const lines = buffer.split(/\r?\n/);
      buffer = done ? '' : (lines.pop() ?? '');
      for (const line of lines) {
        if (line === '') {
          if (data.length > 0) yield data.join('\n');
          data = [];
        } else if (line.startsWith('data:')) {
          data.push(line.slice(5).replace(/^ /, ''));
        }
      }
      if (done) {
        if (data.length > 0) yield data.join('\n');
        return;
      }
    }
  } finally {
    reader.releaseLock();
  }
}

interface ToolAccumulator {
  id: string;
  name: string;
  args: string;
}

function errorMessageOf(payload: Record<string, unknown>): string | null {
  const error = payload.error;
  if (!error) return null;
  if (typeof error === 'string') return error;
  if (typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message);
  }
  return 'Provider returned an error';
}

/**
 * Turn OpenAI Chat Completions stream payloads into events. Text is emitted
 * as it arrives; tool calls (whose arguments arrive in fragments keyed by
 * `index`) are emitted once the stream ends.
 */
export async function* parseOpenAiStream(
  payloads: AsyncIterable<string>,
): AsyncGenerator<RemoteStreamEvent> {
  const tools = new Map<number, ToolAccumulator>();
  for await (const payload of payloads) {
    if (payload.trim() === '[DONE]') break;
    const chunk = JSON.parse(payload) as Record<string, unknown>;
    const error = errorMessageOf(chunk);
    if (error) throw new Error(error);
    const choices = chunk.choices as
      | Array<{
          delta?: {
            content?: string | null;
            tool_calls?: Array<{
              index?: number;
              id?: string;
              function?: { name?: string; arguments?: string };
            }>;
          };
        }>
      | undefined;
    const delta = choices?.[0]?.delta;
    if (!delta) continue;
    if (delta.content) yield { kind: 'text', delta: delta.content };
    for (const call of delta.tool_calls ?? []) {
      const index = call.index ?? tools.size;
      const acc = tools.get(index) ?? { id: '', name: '', args: '' };
      if (call.id) acc.id = call.id;
      if (call.function?.name) acc.name += call.function.name;
      if (call.function?.arguments) acc.args += call.function.arguments;
      tools.set(index, acc);
    }
  }
  for (const [index, acc] of [...tools].sort(([a], [b]) => a - b)) {
    if (!acc.name) continue;
    yield {
      kind: 'tool',
      id: acc.id || `call_${index}`,
      name: acc.name,
      args: acc.args,
    };
  }
}

/**
 * Turn Anthropic Messages stream events into provider-neutral events.
 * `tool_use` blocks are assembled from `input_json_delta` fragments and
 * emitted when their content block stops.
 */
export async function* parseAnthropicStream(
  payloads: AsyncIterable<string>,
): AsyncGenerator<RemoteStreamEvent> {
  const tools = new Map<number, ToolAccumulator>();
  for await (const payload of payloads) {
    const event = JSON.parse(payload) as Record<string, unknown>;
    const type = event.type;
    const index = typeof event.index === 'number' ? event.index : -1;
    if (type === 'error') {
      throw new Error(errorMessageOf(event) ?? 'Provider returned an error');
    }
    if (type === 'content_block_start') {
      const block = event.content_block as {
        type?: string;
        id?: string;
        name?: string;
      };
      if (block?.type === 'tool_use') {
        tools.set(index, {
          id: block.id ?? '',
          name: block.name ?? '',
          args: '',
        });
      }
    } else if (type === 'content_block_delta') {
      const delta = event.delta as {
        type?: string;
        text?: string;
        partial_json?: string;
      };
      if (delta?.type === 'text_delta' && delta.text) {
        yield { kind: 'text', delta: delta.text };
      } else if (delta?.type === 'input_json_delta') {
        const acc = tools.get(index);
        if (acc) acc.args += delta.partial_json ?? '';
      }
    } else if (type === 'content_block_stop') {
      const acc = tools.get(index);
      if (acc) {
        tools.delete(index);
        yield {
          kind: 'tool',
          id: acc.id || `toolu_${index}`,
          name: acc.name,
          args: acc.args || '{}',
        };
      }
    } else if (type === 'message_stop') {
      break;
    }
  }
}

/** Human-readable message for a failed provider response. */
function errorDetail(body: string): string {
  try {
    return errorMessageOf(JSON.parse(body) as Record<string, unknown>) ?? '';
  } catch {
    return body.slice(0, 200);
  }
}

export function describeHttpError(status: number, body: string): string {
  const detail = errorDetail(body);
  const hint =
    status === 401 || status === 403
      ? 'The API key was rejected.'
      : status === 404
        ? 'The base URL or model was not found.'
        : status === 429
          ? 'Rate limit reached. Try again later.'
          : `The provider returned HTTP ${status}.`;
  return detail ? `${hint} ${detail}` : hint;
}

/** Message for a request that never got a response (network / CORS). */
function describeNetworkError(url: string): string {
  return `Could not reach ${hostOf(url)}. Check the base URL, or the provider may block browser requests (CORS).`;
}

/**
 * Send one streaming chat request and yield provider-neutral events.
 *
 * @throws Error with a user-facing message on HTTP or network failure.
 */
export async function* streamRemoteChat(
  target: RemoteChatTarget,
  body: Record<string, unknown>,
  signal?: AbortSignal,
): AsyncGenerator<RemoteStreamEvent> {
  let response: Response;
  try {
    response = await fetch(target.url, {
      method: 'POST',
      headers: target.headers,
      body: JSON.stringify(body),
      signal,
    });
  } catch (err) {
    if (err instanceof DOMException && err.name === 'AbortError') throw err;
    throw new Error(describeNetworkError(target.url), { cause: err });
  }
  if (!response.ok || !response.body) {
    throw new Error(describeHttpError(response.status, await response.text()));
  }
  const payloads = readSseData(response.body);
  yield* target.kind === 'anthropic'
    ? parseAnthropicStream(payloads)
    : parseOpenAiStream(payloads);
}

/** Send a tiny request to check that the provider, key and model work. */
export async function testConnection(
  target: RemoteChatTarget,
): Promise<LlmConnectionTestResult> {
  const body = buildRequestBody(
    target,
    [{ role: 'user', content: 'Reply with OK.' }],
    'You are a connection test. Reply with OK.',
    [],
  );
  body.max_tokens = 16;
  try {
    for await (const event of streamRemoteChat(target, body)) {
      if (event.kind === 'text') break;
    }
    return { ok: true, message: `Connected to ${hostOf(target.url)}.` };
  } catch (err) {
    return {
      ok: false,
      message: err instanceof Error ? err.message : String(err),
    };
  }
}
