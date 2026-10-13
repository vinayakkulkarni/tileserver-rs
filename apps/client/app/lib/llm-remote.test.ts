import { describe, expect, it } from 'vitest';
import type { ModelMessage } from '@tanstack/ai';
import type { OpenAiTool, RemoteStreamEvent } from '~/types/llm';
import {
  buildRequestBody,
  describeHttpError,
  directTarget,
  endpointFor,
  isAllowedBaseUrl,
  parseAnthropicStream,
  parseOpenAiStream,
  proxyTarget,
  readSseData,
  toAnthropicMessages,
  toAnthropicTools,
} from './llm-remote';

/** Build a ReadableStream that delivers `chunks` one by one. */
function streamOf(chunks: string[]): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  return new ReadableStream({
    start(controller) {
      for (const chunk of chunks) controller.enqueue(encoder.encode(chunk));
      controller.close();
    },
  });
}

async function collect<T>(it: AsyncIterable<T>): Promise<T[]> {
  const out: T[] = [];
  for await (const item of it) out.push(item);
  return out;
}

async function* payloads(items: string[]): AsyncGenerator<string> {
  yield* items;
}

const FLY_TO: OpenAiTool = {
  type: 'function',
  function: {
    name: 'fly_to',
    description: 'Fly the camera',
    parameters: { type: 'object', properties: { lng: { type: 'number' } } },
  },
};

describe('isAllowedBaseUrl', () => {
  it('accepts https and loopback http', () => {
    for (const url of [
      'https://api.anthropic.com',
      'https://openrouter.ai/api/v1',
      'http://localhost:11434/v1',
      'http://127.0.0.1:1234',
      'http://[::1]:8080/v1',
    ]) {
      expect(isAllowedBaseUrl(url), url).toBe(true);
    }
  });

  it('rejects remote http, other schemes and malformed input', () => {
    for (const url of [
      'http://example.com/v1',
      'http://localhost.evil.com',
      'javascript:alert(1)',
      'ftp://localhost',
      'not a url',
      '',
    ]) {
      expect(isAllowedBaseUrl(url), url).toBe(false);
    }
  });
});

describe('endpointFor', () => {
  it('appends the provider path once', () => {
    expect(
      endpointFor('openai-compatible', 'https://openrouter.ai/api/v1/'),
    ).toBe('https://openrouter.ai/api/v1/chat/completions');
    expect(endpointFor('anthropic', 'https://api.anthropic.com')).toBe(
      'https://api.anthropic.com/v1/messages',
    );
    expect(endpointFor('anthropic', 'https://gw.example/anthropic/v1')).toBe(
      'https://gw.example/anthropic/v1/messages',
    );
  });
});

describe('directTarget', () => {
  it('puts the OpenAI key in a bearer header, never in the URL', () => {
    const t = directTarget(
      'openai-compatible',
      'https://api.openai.com/v1',
      'gpt',
      'sk-1',
    );
    expect(t.headers.authorization).toBe('Bearer sk-1');
    expect(t.url).toBe('https://api.openai.com/v1/chat/completions');
    expect(t.url).not.toContain('sk-1');
  });

  it('sends Anthropic browser-access headers and x-api-key', () => {
    const t = directTarget('anthropic', '', 'claude', 'sk-ant');
    expect(t.url).toBe('https://api.anthropic.com/v1/messages');
    expect(t.headers['x-api-key']).toBe('sk-ant');
    expect(t.headers['anthropic-version']).toBe('2023-06-01');
    expect(t.headers['anthropic-dangerous-direct-browser-access']).toBe('true');
    expect(t.headers.authorization).toBeUndefined();
  });

  it('omits auth for keyless local servers', () => {
    const t = directTarget(
      'openai-compatible',
      'http://localhost:11434/v1',
      'llama',
      '  ',
    );
    expect(t.headers.authorization).toBeUndefined();
  });
});

describe('buildRequestBody', () => {
  const messages: ModelMessage[] = [{ role: 'user', content: 'Fly to Paris' }];

  it('builds an OpenAI body with system message and tools', () => {
    const t = directTarget(
      'openai-compatible',
      'https://x.test/v1',
      'gpt',
      'k',
    );
    const body = buildRequestBody(t, messages, 'SYS', [FLY_TO]);
    expect(body.model).toBe('gpt');
    expect(body.stream).toBe(true);
    expect(body.messages).toEqual([
      { role: 'system', content: 'SYS' },
      { role: 'user', content: 'Fly to Paris' },
    ]);
    expect(body.tools).toEqual([FLY_TO]);
  });

  it('builds an Anthropic body with top-level system and input_schema tools', () => {
    const t = directTarget('anthropic', '', 'claude', 'k');
    const body = buildRequestBody(t, messages, 'SYS', [FLY_TO]);
    expect(body.system).toBe('SYS');
    expect(body.max_tokens).toBe(1024);
    expect(body.messages).toEqual([
      { role: 'user', content: [{ type: 'text', text: 'Fly to Paris' }] },
    ]);
    expect(body.tools).toEqual([
      {
        name: 'fly_to',
        description: 'Fly the camera',
        input_schema: FLY_TO.function.parameters,
      },
    ]);
  });

  it('leaves the model to the server for the proxy target', () => {
    const body = buildRequestBody(
      proxyTarget('openai-compatible', '/api/llm/chat'),
      messages,
      'S',
      [],
    );
    expect(body.model).toBeUndefined();
    expect(body.tools).toBeUndefined();
  });
});

describe('toAnthropicMessages', () => {
  it('maps tool calls to tool_use and merges tool results into one user turn', () => {
    const messages: ModelMessage[] = [
      { role: 'user', content: 'Go' },
      {
        role: 'assistant',
        content: 'Flying',
        toolCalls: [
          {
            id: 't1',
            type: 'function',
            function: { name: 'fly_to', arguments: '{"lng":2}' },
          },
          {
            id: 't2',
            type: 'function',
            function: { name: 'get_map_state', arguments: '' },
          },
        ],
      },
      { role: 'tool', content: 'ok1', toolCallId: 't1' },
      { role: 'tool', content: 'ok2', toolCallId: 't2' },
    ];
    expect(toAnthropicMessages(messages)).toEqual([
      { role: 'user', content: [{ type: 'text', text: 'Go' }] },
      {
        role: 'assistant',
        content: [
          { type: 'text', text: 'Flying' },
          { type: 'tool_use', id: 't1', name: 'fly_to', input: { lng: 2 } },
          { type: 'tool_use', id: 't2', name: 'get_map_state', input: {} },
        ],
      },
      {
        role: 'user',
        content: [
          { type: 'tool_result', tool_use_id: 't1', content: 'ok1' },
          { type: 'tool_result', tool_use_id: 't2', content: 'ok2' },
        ],
      },
    ]);
  });

  it('converts tools', () => {
    expect(toAnthropicTools([FLY_TO])[0]?.input_schema).toBe(
      FLY_TO.function.parameters,
    );
  });
});

describe('readSseData', () => {
  it('reassembles events split across chunks, with CRLF and comments', async () => {
    const data = await collect(
      readSseData(
        streamOf([
          ': keep-alive\r\n\r\ndata: {"a"',
          ':1}\r\n\r\nevent: x\ndata: line1\ndata: line2\n\n',
          'data: [DONE]',
        ]),
      ),
    );
    expect(data).toEqual(['{"a":1}', 'line1\nline2', '[DONE]']);
  });
});

describe('parseOpenAiStream', () => {
  it('yields text deltas, then tool calls assembled from fragments', async () => {
    const events = await collect(
      parseOpenAiStream(
        payloads([
          '{"choices":[{"delta":{"content":"Fly"}}]}',
          '{"choices":[{"delta":{"content":"ing"}}]}',
          '{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","function":{"name":"fly_to","arguments":"{\\"lng\\":"}}]}}]}',
          '{"choices":[{"delta":{"tool_calls":[{"index":1,"id":"c2","function":{"name":"get_map_state","arguments":"{}"}}]}}]}',
          '{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"2.35}"}}]}}]}',
          '[DONE]',
          '{"choices":[{"delta":{"content":"ignored after DONE"}}]}',
        ]),
      ),
    );
    expect(events).toEqual<RemoteStreamEvent[]>([
      { kind: 'text', delta: 'Fly' },
      { kind: 'text', delta: 'ing' },
      { kind: 'tool', id: 'c1', name: 'fly_to', args: '{"lng":2.35}' },
      { kind: 'tool', id: 'c2', name: 'get_map_state', args: '{}' },
    ]);
  });

  it('throws the provider error message mid-stream', async () => {
    await expect(
      collect(
        parseOpenAiStream(payloads(['{"error":{"message":"quota exceeded"}}'])),
      ),
    ).rejects.toThrow('quota exceeded');
  });
});

describe('parseAnthropicStream', () => {
  it('yields text_delta text and assembles tool_use input', async () => {
    const events = await collect(
      parseAnthropicStream(
        payloads([
          '{"type":"message_start","message":{"id":"m"}}',
          '{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}',
          '{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Sure"}}',
          '{"type":"content_block_stop","index":0}',
          '{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_1","name":"fly_to","input":{}}}',
          '{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\\"lng\\": 2"}}',
          '{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":", \\"lat\\": 48}"}}',
          '{"type":"content_block_stop","index":1}',
          '{"type":"message_delta","delta":{"stop_reason":"tool_use"}}',
          '{"type":"message_stop"}',
        ]),
      ),
    );
    expect(events).toEqual<RemoteStreamEvent[]>([
      { kind: 'text', delta: 'Sure' },
      {
        kind: 'tool',
        id: 'toolu_1',
        name: 'fly_to',
        args: '{"lng": 2, "lat": 48}',
      },
    ]);
  });

  it('defaults empty tool input to {}', async () => {
    const events = await collect(
      parseAnthropicStream(
        payloads([
          '{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"t","name":"get_map_state"}}',
          '{"type":"content_block_stop","index":0}',
        ]),
      ),
    );
    expect(events).toEqual([
      { kind: 'tool', id: 't', name: 'get_map_state', args: '{}' },
    ]);
  });

  it('throws on an error event', async () => {
    await expect(
      collect(
        parseAnthropicStream(
          payloads([
            '{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}',
          ]),
        ),
      ),
    ).rejects.toThrow('Overloaded');
  });
});

describe('describeHttpError', () => {
  it('explains common statuses with the provider detail', () => {
    expect(
      describeHttpError(401, '{"error":{"message":"invalid x-api-key"}}'),
    ).toBe('The API key was rejected. invalid x-api-key');
    expect(describeHttpError(404, '')).toBe(
      'The base URL or model was not found.',
    );
    expect(describeHttpError(429, 'busy')).toBe(
      'Rate limit reached. Try again later. busy',
    );
    expect(describeHttpError(500, '')).toBe('The provider returned HTTP 500.');
  });
});
