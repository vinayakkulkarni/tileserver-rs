/**
 * useLlmChat Composable
 *
 * Bridges WebLLM (browser-local LLM) with TanStack AI Vue's useChat.
 * Uses the `stream()` adapter to convert WebLLM's OpenAI-compatible
 * streaming output to AG-UI protocol events.
 *
 * WebLLM restricts native tool calling to Hermes models only. All Qwen3
 * models use text-based [MAP_ACTION] blocks + parseToolIntentsFromText().
 *
 * @see https://tanstack.com/ai/latest/docs/guides/client-tools
 * @see https://webllm.mlc.ai/
 */

import { useChat, stream } from '@tanstack/ai-vue';
import { EventType, convertMessagesToModelMessages } from '@tanstack/ai';
import type { ModelMessage, StreamChunk } from '@tanstack/ai';
import type {
  ChatCompletionMessageParam,
  ChatCompletionMessageToolCall,
} from '@mlc-ai/web-llm';
import type { Map as MaplibreMap } from 'maplibre-gl';
import type { OverlayLayer } from '~/types/file-upload';
import {
  createMapClientTools,
  createServerClientTools,
  WEBLLM_TOOLS,
  WEBLLM_SERVER_TOOLS,
} from '~/lib/map-tools';
import { chatCollection } from '~/lib/chat-db';
import { buildRequestBody, streamRemoteChat } from '~/lib/llm-remote';
import type { RemoteChatTarget, StoredToolCall } from '~/types/llm';
import { useLlmProvider } from './use-llm-provider';

/** System prompt for models with WebLLM native tool calling enabled. */
const SYSTEM_PROMPT_WITH_TOOLS = `You are a helpful map assistant embedded in tileserver-rs, a vector tile server.
You can help users explore the map by flying to locations, adjusting the view, querying features, and modifying styles.

Available tools:
- fly_to: Navigate to a specific location
- fit_bounds: Fit the view to a bounding box
- get_map_state: Get current map state (center, zoom, layers)
- set_layer_visibility: Show or hide a layer
- set_layer_paint: Change a layer's paint property (color, opacity, etc.)
- set_layer_filter: Apply a filter expression to a layer
- query_rendered_features: Query visible features in the viewport
- add_highlight: Temporarily highlight features matching a filter
- generate_style: Apply multiple style changes from a description

Overlay tools (user-dropped files):
- get_overlays: List all user-dropped file overlays on the map (file names, formats, feature counts, visibility)

Server-side tools (query tile data):
- get_source_schema: Get available layers, fields, zoom range for a data source
- get_source_stats: Get bounds, attribution, and layer count for a data source
- spatial_query: Query features from a data source within a bounding box

When users ask to see a place, use fly_to with coordinates.
When users ask to see a region, use fit_bounds with a bounding box.
Use get_map_state to understand what the user is currently looking at before making changes.
Use get_source_schema or get_source_stats when users ask about available data or layers.
Use spatial_query when users want to find specific features in the data.
Use get_overlays when users ask about overlays, dropped files, or uploaded data on the map.
Keep responses concise and helpful. You're a map expert.`;

/** System prompt for non-tool models — uses [MAP_ACTION] JSON blocks parsed client-side. */
const SYSTEM_PROMPT_NO_TOOLS = `You are a helpful map assistant embedded in tileserver-rs, a vector tile server.
You can help users explore the map by flying to locations, adjusting the view, and answering questions about geography.

IMPORTANT: You can control the map directly. When a user asks to navigate somewhere, you MUST include a map action block in your response using this exact format:

[MAP_ACTION]{"action":"fly_to","lng":<longitude>,"lat":<latitude>,"zoom":<zoom_level>}[/MAP_ACTION]

Available actions:
- fly_to: Navigate to a location. Required: lng, lat. Optional: zoom (default 12), bearing (default 0), pitch (default 0).
  Example: [MAP_ACTION]{"action":"fly_to","lng":2.3522,"lat":48.8566,"zoom":13}[/MAP_ACTION]
- fit_bounds: Fit map to a bounding box. Required: west, south, east, north. Optional: padding (default 50).
  Example: [MAP_ACTION]{"action":"fit_bounds","west":-5.0,"south":42.0,"east":9.5,"north":51.0}[/MAP_ACTION]

ALWAYS include the [MAP_ACTION] block when the user asks to go somewhere, fly to a place, or show a location. Put it at the END of your response after your text explanation.
Keep responses concise and helpful. You're a map expert.`;

/**
 * Convert TanStack model messages to WebLLM's OpenAI-compatible format.
 * Tool calls and tool results are only forwarded for tool-capable models.
 */
function toWebLlmMessages(
  messages: ModelMessage[],
  systemPrompt: string,
  includeTools: boolean,
): ChatCompletionMessageParam[] {
  const result: ChatCompletionMessageParam[] = [
    { role: 'system', content: systemPrompt },
  ];

  for (const msg of messages) {
    const content = contentToText(msg.content);
    if (msg.role === 'user') {
      result.push({ role: 'user', content });
    } else if (msg.role === 'assistant') {
      const toolCalls: ChatCompletionMessageToolCall[] =
        includeTools && msg.toolCalls
          ? msg.toolCalls.map((call) => ({
              id: call.id,
              type: 'function',
              function: {
                name: call.function.name,
                arguments: call.function.arguments,
              },
            }))
          : [];
      result.push(
        toolCalls.length > 0
          ? { role: 'assistant', content, tool_calls: toolCalls }
          : { role: 'assistant', content },
      );
    } else if (includeTools && msg.toolCallId) {
      result.push({ role: 'tool', content, tool_call_id: msg.toolCallId });
    }
  }

  return result;
}

/**
 * Flatten a model message's content to plain text (non-text parts are dropped).
 */
function contentToText(content: ModelMessage['content']): string {
  if (typeof content === 'string') return content;
  if (!content) return '';
  return content
    .map((part) => (part.type === 'text' ? part.content : ''))
    .join('');
}

/**
 * Parse [MAP_ACTION]{...}[/MAP_ACTION] blocks from LLM response text.
 * Used as fallback for non-tool models (Qwen).
 */
function parseMapActions(
  text: string,
): Array<{ action: string } & Record<string, unknown>> {
  const regex = /\[MAP_ACTION\](\{[\s\S]*?\})\[\/MAP_ACTION\]/g;
  const actions: Array<{ action: string } & Record<string, unknown>> = [];
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    const json = match[1];
    if (json === undefined) continue;
    try {
      const parsed = JSON.parse(json) as { action: string } & Record<
        string,
        unknown
      >;
      if (parsed.action) {
        actions.push(parsed);
      }
    } catch {
      // Invalid JSON in action block — skip
    }
  }

  return actions;
}

/**
 * Parse the first capture group of a regex match as a number.
 * Returns undefined when the pattern did not match.
 */
function parseCapturedNumber(
  match: RegExpMatchArray | null,
): number | undefined {
  const raw = match?.[1];
  return raw === undefined ? undefined : Number.parseFloat(raw);
}

/**
 * Parse tool intents from the model's natural language response.
 * When WebLLM's tool-calling parser fails, the model writes tool invocations
 * as plain text (e.g., "south: 30.7, west: -28.9"). This extracts them
 * so we can execute the intended action on the map.
 */
function parseToolIntentsFromText(
  text: string,
): Array<{ action: string } & Record<string, unknown>> {
  const intents: Array<{ action: string } & Record<string, unknown>> = [];

  // 1. Try <tool_call> JSON blocks (Hermes/Qwen3 format)
  const toolCallRegex = /<tool_call>\s*(\{[\s\S]*?\})\s*<\/tool_call>/g;
  let tcMatch: RegExpExecArray | null;
  while ((tcMatch = toolCallRegex.exec(text)) !== null) {
    const json = tcMatch[1];
    if (json === undefined) continue;
    try {
      const parsed = JSON.parse(json) as {
        name?: string;
        arguments?: Record<string, unknown>;
      };
      if (parsed.name && parsed.arguments) {
        intents.push({ action: parsed.name, ...parsed.arguments });
      }
    } catch {
      // Invalid JSON — skip
    }
  }
  if (intents.length > 0) return intents;

  // 2. Extract all [number, number] coordinate arrays from text
  //    The model writes coordinates in many formats:
  //    - southwest: [-16.33, 35.84], northeast: [18.89, 47.11]
  //    - fit_bounds([12.49, 41.88], [12.53, 41.92])
  //    - fly_to([12.49, 41.89])
  //    - [12.49, 41.89]
  const coordArrays: Array<[number, number]> = [];
  const arrayRegex = /\[\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*\]/g;
  let arrMatch: RegExpExecArray | null;
  while ((arrMatch = arrayRegex.exec(text)) !== null) {
    const first = arrMatch[1];
    const second = arrMatch[2];
    if (first === undefined || second === undefined) continue;
    coordArrays.push([Number.parseFloat(first), Number.parseFloat(second)]);
  }

  const mentionsBounds = /fit_bounds|bounds|bounding|southwest|northeast/i.test(
    text,
  );

  // Two coordinate arrays + bounds context → fit_bounds
  const [sw, ne] = coordArrays;
  if (sw && ne && mentionsBounds) {
    intents.push({
      action: 'fit_bounds',
      west: Math.min(sw[0], ne[0]),
      south: Math.min(sw[1], ne[1]),
      east: Math.max(sw[0], ne[0]),
      north: Math.max(sw[1], ne[1]),
    });
    return intents;
  }

  // Single coordinate array → fly_to
  if (sw) {
    const [lng, lat] = sw;
    // Sanity check: valid geographic coordinates
    if (Math.abs(lng) <= 180 && Math.abs(lat) <= 90) {
      const zoom = parseCapturedNumber(text.match(/zoom\D*(\d+(?:\.\d+)?)/i));
      intents.push({ action: 'fly_to', lng, lat, zoom: zoom ?? 10 });
      return intents;
    }
  }

  // 3. Cardinal direction coordinates: south: 35.8, west: -16.3, etc.
  const south = parseCapturedNumber(
    text.match(/south[:\s]+(-?\d+(?:\.\d+)?)/i),
  );
  const west = parseCapturedNumber(text.match(/west[:\s]+(-?\d+(?:\.\d+)?)/i));
  const north = parseCapturedNumber(
    text.match(/north[:\s]+(-?\d+(?:\.\d+)?)/i),
  );
  const east = parseCapturedNumber(text.match(/east[:\s]+(-?\d+(?:\.\d+)?)/i));
  if (
    south !== undefined &&
    west !== undefined &&
    north !== undefined &&
    east !== undefined
  ) {
    intents.push({ action: 'fit_bounds', west, south, east, north });
    return intents;
  }

  // 4. Labeled lng/lat: lng: 12.49, lat: 41.89
  const labeledLng = parseCapturedNumber(
    text.match(/(?:lng|longitude)[:\s]+(-?\d+(?:\.\d+)?)/i),
  );
  const labeledLat = parseCapturedNumber(
    text.match(/(?:lat|latitude)[:\s]+(-?\d+(?:\.\d+)?)/i),
  );
  if (labeledLng !== undefined && labeledLat !== undefined) {
    const zoom = parseCapturedNumber(text.match(/zoom\D*(\d+(?:\.\d+)?)/i));
    intents.push({
      action: 'fly_to',
      lng: labeledLng,
      lat: labeledLat,
      ...(zoom !== undefined ? { zoom } : {}),
    });
    return intents;
  }

  // 5. Bare coordinate pair: "41.8902, 12.4922" (lat, lng natural order)
  const bareCoords = text.match(/(-?\d+\.\d{2,})\s*,\s*(-?\d+\.\d{2,})/);
  const bareFirst = bareCoords?.[1];
  const bareSecond = bareCoords?.[2];
  if (bareFirst !== undefined && bareSecond !== undefined) {
    const first = Number.parseFloat(bareFirst);
    const second = Number.parseFloat(bareSecond);
    // Heuristic: if |first| > 90, it's lng,lat; otherwise lat,lng
    const [lat, lng] = Math.abs(first) > 90 ? [second, first] : [first, second];
    if (Math.abs(lng) <= 180 && Math.abs(lat) <= 90) {
      const zoom = parseCapturedNumber(text.match(/zoom\D*(\d+(?:\.\d+)?)/i));
      intents.push({ action: 'fly_to', lng, lat, zoom: zoom ?? 6 });
      return intents;
    }
  }

  return intents;
}

/**
 * Execute a text-based map action (fallback for non-tool models).
 * Only handles fly_to and fit_bounds — the basic text-based tools.
 */
function executeFallbackAction(
  action: { action: string } & Record<string, unknown>,
  map: MaplibreMap | null,
): string {
  if (!map) return 'Map not available';

  switch (action.action) {
    case 'fly_to': {
      const lng = Number(action.lng);
      const lat = Number(action.lat);
      const zoom = Number(action.zoom ?? 12);
      const bearing = Number(action.bearing ?? 0);
      const pitch = Number(action.pitch ?? 0);
      map.flyTo({ center: [lng, lat], zoom, bearing, pitch, duration: 2000 });
      return `Flying to [${lng}, ${lat}] at zoom ${zoom}`;
    }
    case 'fit_bounds': {
      const west = Number(action.west);
      const south = Number(action.south);
      const east = Number(action.east);
      const north = Number(action.north);
      const padding = Number(action.padding ?? 50);
      map.fitBounds(
        [
          [west, south],
          [east, north],
        ],
        { padding, duration: 2000 },
      );
      return `Fitting to bounds [${west},${south},${east},${north}]`;
    }
    default:
      return `Unknown action: ${action.action}`;
  }
}

/**
 * Stream one turn from a remote provider (bring-your-own-key or the server
 * proxy) as AG-UI events. Remote models use native tool calling, so the
 * full map + server tool set is sent and tool calls are emitted as
 * TOOL_CALL_* events for useChat to execute.
 */
async function* streamRemoteTurn(
  target: RemoteChatTarget,
  messages: ModelMessage[],
  messageId: string,
): AsyncGenerator<StreamChunk> {
  const body = buildRequestBody(target, messages, SYSTEM_PROMPT_WITH_TOOLS, [
    ...WEBLLM_TOOLS,
    ...WEBLLM_SERVER_TOOLS,
  ]);
  const toolCalls: Array<{ id: string; name: string; args: string }> = [];

  try {
    for await (const event of streamRemoteChat(target, body)) {
      if (event.kind === 'text') {
        yield {
          type: EventType.TEXT_MESSAGE_CONTENT,
          messageId,
          delta: event.delta,
          timestamp: Date.now(),
        };
      } else {
        toolCalls.push(event);
      }
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    yield {
      type: EventType.TEXT_MESSAGE_CONTENT,
      messageId,
      delta: `\n\n⚠️ ${message}`,
      timestamp: Date.now(),
    };
  }

  yield { type: EventType.TEXT_MESSAGE_END, messageId, timestamp: Date.now() };

  for (const call of toolCalls) {
    yield {
      type: EventType.TOOL_CALL_START,
      toolCallId: call.id,
      toolCallName: call.name,
      timestamp: Date.now(),
    };
    yield {
      type: EventType.TOOL_CALL_ARGS,
      toolCallId: call.id,
      delta: call.args,
      timestamp: Date.now(),
    };
    yield {
      type: EventType.TOOL_CALL_END,
      toolCallId: call.id,
      timestamp: Date.now(),
    };
  }
}

/**
 * Composable for LLM chat with map tool integration.
 *
 * Two code paths based on `supportsTools`:
 *   - true: native WebLLM tool calls → useChat auto-executes client tools
 *   - false: text-based [MAP_ACTION] blocks → parsed after streaming
 *
 * @param mapRef - Ref to the MapLibre GL map instance
 * @param overlaysRef - Ref to the overlay layers from file drops
 */
export function useLlmChat(
  mapRef: Ref<MaplibreMap | null>,
  overlaysRef: Ref<readonly OverlayLayer[]>,
) {
  const { engine, status, selectedModel } = useLlmEngine();
  const { isRemote, remoteTarget } = useLlmProvider();

  // Create client tools bound to the map ref and overlays
  const clientTools = createMapClientTools(
    () => mapRef.value,
    () => overlaysRef.value,
  );
  const serverTools = createServerClientTools();

  /**
   * Create the connection adapter that bridges WebLLM → AG-UI events.
   * The `stream()` helper converts an async generator into a ConnectionAdapter.
   *
   * For tool-capable models:
   *   - Passes WEBLLM_TOOLS to engine.chat.completions.create()
   *   - Yields TOOL_CALL_START/ARGS/END events for tool calls
   *   - useChat auto-executes matching client tools
   *   - useChat re-invokes this adapter with updated messages (incl. tool results)
   *
   * For non-tool models:
   *   - Text-only streaming
   *   - [MAP_ACTION] blocks parsed and executed after streaming
   */
  // AG-UI thread id: one conversation per composable instance.
  const threadId = crypto.randomUUID();

  const connection = stream(
    async function* (messages): AsyncGenerator<StreamChunk> {
      // --- Remote provider (bring your own key, or the server proxy) ---
      if (isRemote.value) {
        const target = remoteTarget.value;
        if (!target) {
          throw new Error(
            'Set up the remote provider in the Map AI settings first.',
          );
        }
        const runId = crypto.randomUUID();
        const messageId = crypto.randomUUID();
        yield {
          type: EventType.RUN_STARTED,
          threadId,
          runId,
          timestamp: Date.now(),
        };
        yield {
          type: EventType.TEXT_MESSAGE_START,
          messageId,
          role: 'assistant' as const,
          timestamp: Date.now(),
        };
        yield* streamRemoteTurn(
          target,
          convertMessagesToModelMessages(messages),
          messageId,
        );
        yield {
          type: EventType.RUN_FINISHED,
          threadId,
          runId,
          finishReason: 'stop',
          timestamp: Date.now(),
        };
        return;
      }

      const currentEngine = engine.value;
      if (!currentEngine) {
        throw new Error(
          'LLM engine not initialized. Please wait for model to load.',
        );
      }

      const runId = crypto.randomUUID();
      const messageId = crypto.randomUUID();
      const useTools = selectedModel.value.supportsTools;
      const systemPrompt = useTools
        ? SYSTEM_PROMPT_WITH_TOOLS
        : SYSTEM_PROMPT_NO_TOOLS;

      yield {
        type: EventType.RUN_STARTED,
        threadId,
        runId,
        timestamp: Date.now(),
      };
      yield {
        type: EventType.TEXT_MESSAGE_START,
        messageId,
        role: 'assistant' as const,
        timestamp: Date.now(),
      };

      // Accumulate text outside try so catch block can parse tool intents from it
      let accumulatedText = '';

      try {
        // Convert UI/model messages → OpenAI-compatible format for WebLLM
        const openaiMessages = toWebLlmMessages(
          convertMessagesToModelMessages(messages),
          systemPrompt,
          useTools,
        );

        const response = await currentEngine.chat.completions.create({
          messages: openaiMessages,
          stream: true,
          ...(useTools
            ? { tools: [...WEBLLM_TOOLS, ...WEBLLM_SERVER_TOOLS] }
            : {}),
          temperature: 0.7,
          max_tokens: 1024,
        });

        let pendingToolCalls: Array<{
          id: string;
          function: { name: string; arguments: string };
        }> = [];

        for await (const chunk of response) {
          const choice = chunk.choices[0];
          if (!choice) continue;

          // Stream text content
          const delta = choice.delta?.content;
          if (delta) {
            accumulatedText += delta;
            yield {
              type: EventType.TEXT_MESSAGE_CONTENT,
              messageId,
              delta,
              timestamp: Date.now(),
            };
          }

          // Collect tool calls (WebLLM sends them on the last chunk only)
          if (choice.delta?.tool_calls) {
            for (const tc of choice.delta.tool_calls) {
              if (tc.id && tc.function) {
                pendingToolCalls.push({
                  id: tc.id,
                  function: {
                    name: tc.function.name ?? '',
                    arguments: tc.function.arguments ?? '',
                  },
                });
              }
            }
          }
        }

        // --- Native tool calls (models with supportsTools: true) ---
        if (pendingToolCalls.length > 0) {
          // End text message, then emit tool call events
          yield {
            type: EventType.TEXT_MESSAGE_END,
            messageId,
            timestamp: Date.now(),
          };

          for (const toolCall of pendingToolCalls) {
            yield {
              type: EventType.TOOL_CALL_START,
              toolCallId: toolCall.id,
              toolCallName: toolCall.function.name,
              timestamp: Date.now(),
            };
            yield {
              type: EventType.TOOL_CALL_ARGS,
              toolCallId: toolCall.id,
              delta: toolCall.function.arguments,
              timestamp: Date.now(),
            };
            yield {
              type: EventType.TOOL_CALL_END,
              toolCallId: toolCall.id,
              timestamp: Date.now(),
            };
          }

          // useChat will auto-execute matching client tools and re-invoke this adapter
          pendingToolCalls = [];
        } else if (!useTools && accumulatedText) {
          // --- Text-based action parsing (Qwen / non-tool models) ---
          const actions = parseMapActions(accumulatedText);
          for (const action of actions) {
            const resultText = executeFallbackAction(action, mapRef.value);
            yield {
              type: EventType.TEXT_MESSAGE_CONTENT,
              messageId,
              delta: `\n\n✅ ${resultText}`,
              timestamp: Date.now(),
            };
          }
          yield {
            type: EventType.TEXT_MESSAGE_END,
            messageId,
            timestamp: Date.now(),
          };
        } else {
          yield {
            type: EventType.TEXT_MESSAGE_END,
            messageId,
            timestamp: Date.now(),
          };
        }
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : '';
        const isToolParseError =
          errorMessage.includes('parsing outputMessage for function calling') ||
          errorMessage.includes('Got outputMessage:');
        if (isToolParseError) {
          // WebLLM tool-call parse failed — extract intents from accumulated text
          console.warn(
            '[LLM] Tool-calling parse failed, parsing intents from text',
          );
          const intents = parseToolIntentsFromText(accumulatedText);
          if (intents.length > 0) {
            for (const intent of intents) {
              const resultText = executeFallbackAction(intent, mapRef.value);
              yield {
                type: EventType.TEXT_MESSAGE_CONTENT,
                messageId,
                delta: `\n\n✅ ${resultText}`,
                timestamp: Date.now(),
              };
            }
          } else {
            // No parseable intents — inject real map state as fallback
            const map = mapRef.value;
            if (map) {
              const center = map.getCenter();
              const zoom = Math.round(map.getZoom() * 100) / 100;
              const bearing = Math.round(map.getBearing());
              const pitch = Math.round(map.getPitch());
              const layers =
                map
                  .getStyle()
                  ?.layers?.filter(
                    (l) => map.getLayoutProperty(l.id, 'visibility') !== 'none',
                  )
                  ?.map((l) => l.id)
                  ?.slice(0, 20) ?? [];
              const stateBlock = [
                '\n\n---',
                '**Current map state:**',
                `- Center: ${center.lng.toFixed(4)}, ${center.lat.toFixed(4)}`,
                `- Zoom: ${zoom}`,
                `- Bearing: ${bearing}°, Pitch: ${pitch}°`,
                `- Visible layers (${layers.length}): ${layers.slice(0, 10).join(', ')}${layers.length > 10 ? '...' : ''}`,
              ].join('\n');
              yield {
                type: EventType.TEXT_MESSAGE_CONTENT,
                messageId,
                delta: stateBlock,
                timestamp: Date.now(),
              };
            }
          }
        } else if (errorMessage) {
          // Genuine error (engine not ready, network, etc.) — show a clean user message
          yield {
            type: EventType.TEXT_MESSAGE_CONTENT,
            messageId,
            delta: '\n\nSomething went wrong. Please try again.',
            timestamp: Date.now(),
          };
        }
        yield {
          type: EventType.TEXT_MESSAGE_END,
          messageId,
          timestamp: Date.now(),
        };
      }

      yield {
        type: EventType.RUN_FINISHED,
        threadId,
        runId,
        finishReason: 'stop',
        timestamp: Date.now(),
      };
    },
  );

  const chat = useChat({
    connection,
    // Pass client + server tools for auto-execution by useChat
    // When the adapter yields TOOL_CALL events, useChat matches them
    // to these client tools, executes them, and re-invokes the adapter.
    tools: [...clientTools, ...serverTools],
    // Persist completed messages to TanStack DB (localStorage-backed)
    onFinish: (message) => {
      if (!import.meta.client) return;
      // Flatten text + tool-call parts for localStorage-friendly storage
      let textContent = '';
      const toolCalls: StoredToolCall[] = [];
      for (const part of message.parts) {
        if (part.type === 'text') {
          textContent += part.content;
        } else if (part.type === 'tool-call') {
          toolCalls.push({
            id: part.id,
            name: part.name,
            args: part.arguments,
          });
        }
      }
      chatCollection.insert({
        id: message.id,
        role: message.role as 'user' | 'assistant',
        content: textContent,
        timestamp: Date.now(),
        ...(toolCalls.length > 0 ? { toolCalls } : {}),
      });
    },
  });

  return {
    ...chat,
    engineStatus: status,
  };
}
