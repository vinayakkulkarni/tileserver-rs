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
import type { UIMessage } from '@tanstack/ai-vue';
import type { MessagePart } from '@tanstack/ai';
import type { Map as MaplibreMap } from 'maplibre-gl';
import type { OverlayLayer } from '~/types/file-upload';
import {
  createMapClientTools,
  createServerClientTools,
  WEBLLM_TOOLS,
  WEBLLM_SERVER_TOOLS,
} from '~/lib/map-tools';
import { chatCollection } from '~/lib/chat-db';
import type { ChatWithEngineStatus, StoredToolCall } from '~/types/llm';

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
 * Convert TanStack AI UIMessage parts to OpenAI-format messages for WebLLM.
 *
 * Handles three part types:
 * - text → { role, content }
 * - tool-call → { role: 'assistant', tool_calls: [...] }
 * - tool-result → { role: 'tool', tool_call_id, content }
 */
function convertMessagesToOpenAI(
  messages: UIMessage[],
  systemPrompt: string,
): Array<{
  role: string;
  content: string;
  tool_calls?: Array<{
    id: string;
    type: 'function';
    function: { name: string; arguments: string };
  }>;
  tool_call_id?: string;
}> {
  const result: Array<{
    role: string;
    content: string;
    tool_calls?: Array<{
      id: string;
      type: 'function';
      function: { name: string; arguments: string };
    }>;
    tool_call_id?: string;
  }> = [{ role: 'system', content: systemPrompt }];

  for (const msg of messages) {
    // Group parts by type for each message
    const textParts: string[] = [];
    const toolCallParts: Array<{
      id: string;
      type: 'function';
      function: { name: string; arguments: string };
    }> = [];
    const toolResultParts: Array<{ toolCallId: string; result: unknown }> = [];

    for (const part of msg.parts as MessagePart[]) {
      switch (part.type) {
        case 'text':
          textParts.push(part.content);
          break;
        case 'tool-call':
          toolCallParts.push({
            id: part.toolCallId,
            type: 'function',
            function: {
              name: part.toolName,
              arguments: JSON.stringify(part.args),
            },
          });
          break;
        case 'tool-result':
          toolResultParts.push({
            toolCallId: part.toolCallId,
            result: part.result,
          });
          break;
      }
    }

    // Emit text content as a regular message
    if (textParts.length > 0) {
      result.push({ role: msg.role, content: textParts.join('') });
    }

    // Emit tool calls as assistant message with tool_calls
    if (toolCallParts.length > 0) {
      result.push({
        role: 'assistant',
        content: '',
        tool_calls: toolCallParts,
      });
    }

    // Emit tool results as individual tool messages
    for (const tr of toolResultParts) {
      result.push({
        role: 'tool',
        content: JSON.stringify(tr.result),
        tool_call_id: tr.toolCallId,
      });
    }
  }

  return result;
}

/**
 * Extract text content from a UIMessage (for non-tool fallback)
 */
function extractText(message: UIMessage): string {
  return message.parts
    .filter((p): p is { type: 'text'; content: string } => p.type === 'text')
    .map((p) => p.content)
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
    try {
      const parsed = JSON.parse(match[1]) as { action: string } & Record<
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
    try {
      const parsed = JSON.parse(tcMatch[1]) as {
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
    coordArrays.push([
      Number.parseFloat(arrMatch[1]),
      Number.parseFloat(arrMatch[2]),
    ]);
  }

  const mentionsBounds = /fit_bounds|bounds|bounding|southwest|northeast/i.test(
    text,
  );

  // Two coordinate arrays + bounds context → fit_bounds
  if (coordArrays.length >= 2 && mentionsBounds) {
    const [sw, ne] = coordArrays;
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
  if (coordArrays.length >= 1) {
    const [lng, lat] = coordArrays[0];
    // Sanity check: valid geographic coordinates
    if (Math.abs(lng) <= 180 && Math.abs(lat) <= 90) {
      const zoomMatch = text.match(/zoom\D*(\d+(?:\.\d+)?)/i);
      intents.push({
        action: 'fly_to',
        lng,
        lat,
        ...(zoomMatch
          ? { zoom: Number.parseFloat(zoomMatch[1]) }
          : { zoom: 10 }),
      });
      return intents;
    }
  }

  // 3. Cardinal direction coordinates: south: 35.8, west: -16.3, etc.
  const south = text.match(/south[:\s]+(-?\d+(?:\.\d+)?)/i);
  const west = text.match(/west[:\s]+(-?\d+(?:\.\d+)?)/i);
  const north = text.match(/north[:\s]+(-?\d+(?:\.\d+)?)/i);
  const east = text.match(/east[:\s]+(-?\d+(?:\.\d+)?)/i);
  if (south && west && north && east) {
    intents.push({
      action: 'fit_bounds',
      west: Number.parseFloat(west[1]),
      south: Number.parseFloat(south[1]),
      east: Number.parseFloat(east[1]),
      north: Number.parseFloat(north[1]),
    });
    return intents;
  }

  // 4. Labeled lng/lat: lng: 12.49, lat: 41.89
  const lngMatch = text.match(/(?:lng|longitude)[:\s]+(-?\d+(?:\.\d+)?)/i);
  const latMatch = text.match(/(?:lat|latitude)[:\s]+(-?\d+(?:\.\d+)?)/i);
  if (lngMatch && latMatch) {
    const zoomMatch = text.match(/zoom\D*(\d+(?:\.\d+)?)/i);
    intents.push({
      action: 'fly_to',
      lng: Number.parseFloat(lngMatch[1]),
      lat: Number.parseFloat(latMatch[1]),
      ...(zoomMatch ? { zoom: Number.parseFloat(zoomMatch[1]) } : {}),
    });
    return intents;
  }

  // 5. Bare coordinate pair: "41.8902, 12.4922" (lat, lng natural order)
  const bareCoords = text.match(/(-?\d+\.\d{2,})\s*,\s*(-?\d+\.\d{2,})/);
  if (bareCoords) {
    const first = Number.parseFloat(bareCoords[1]);
    const second = Number.parseFloat(bareCoords[2]);
    // Heuristic: if |first| > 90, it's lng,lat; otherwise lat,lng
    const [lat, lng] = Math.abs(first) > 90 ? [second, first] : [first, second];
    if (Math.abs(lng) <= 180 && Math.abs(lat) <= 90) {
      const zoomMatch = text.match(/zoom\D*(\d+(?:\.\d+)?)/i);
      intents.push({
        action: 'fly_to',
        lng,
        lat,
        ...(zoomMatch
          ? { zoom: Number.parseFloat(zoomMatch[1]) }
          : { zoom: 6 }),
      });
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
): ChatWithEngineStatus {
  const { engine, status, selectedModel } = useLlmEngine();

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
  const connection = stream(async function* (messages: UIMessage[]) {
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

    yield { type: 'RUN_STARTED' as const, runId, timestamp: Date.now() };
    yield {
      type: 'TEXT_MESSAGE_START' as const,
      messageId,
      role: 'assistant' as const,
      timestamp: Date.now(),
    };

    // Accumulate text outside try so catch block can parse tool intents from it
    let accumulatedText = '';

    try {
      // Convert UIMessage[] → OpenAI format for WebLLM
      const openaiMessages = useTools
        ? convertMessagesToOpenAI(messages, systemPrompt)
        : [
            { role: 'system', content: systemPrompt },
            ...messages.map((m) => ({ role: m.role, content: extractText(m) })),
          ];

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
            type: 'TEXT_MESSAGE_CONTENT' as const,
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
          type: 'TEXT_MESSAGE_END' as const,
          messageId,
          timestamp: Date.now(),
        };

        for (const toolCall of pendingToolCalls) {
          yield {
            type: 'TOOL_CALL_START' as const,
            toolCallId: toolCall.id,
            toolName: toolCall.function.name,
            timestamp: Date.now(),
          };
          yield {
            type: 'TOOL_CALL_ARGS' as const,
            toolCallId: toolCall.id,
            delta: toolCall.function.arguments,
            timestamp: Date.now(),
          };
          yield {
            type: 'TOOL_CALL_END' as const,
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
            type: 'TEXT_MESSAGE_CONTENT' as const,
            messageId,
            delta: `\n\n✅ ${resultText}`,
            timestamp: Date.now(),
          };
        }
        yield {
          type: 'TEXT_MESSAGE_END' as const,
          messageId,
          timestamp: Date.now(),
        };
      } else {
        yield {
          type: 'TEXT_MESSAGE_END' as const,
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
              type: 'TEXT_MESSAGE_CONTENT' as const,
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
              type: 'TEXT_MESSAGE_CONTENT' as const,
              messageId,
              delta: stateBlock,
              timestamp: Date.now(),
            };
          }
        }
      } else if (errorMessage) {
        // Genuine error (engine not ready, network, etc.) — show a clean user message
        yield {
          type: 'TEXT_MESSAGE_CONTENT' as const,
          messageId,
          delta: '\n\nSomething went wrong. Please try again.',
          timestamp: Date.now(),
        };
      }
      yield {
        type: 'TEXT_MESSAGE_END' as const,
        messageId,
        timestamp: Date.now(),
      };
    }

    yield {
      type: 'RUN_FINISHED' as const,
      runId,
      finishReason: 'stop' as const,
      timestamp: Date.now(),
    };
  });

  const chat = useChat({
    connection,
    // Pass client + server tools for auto-execution by useChat
    // When the adapter yields TOOL_CALL events, useChat matches them
    // to these client tools, executes them, and re-invokes the adapter.
    tools: [...clientTools, ...serverTools],
    // Persist completed messages to TanStack DB (localStorage-backed)
    onFinish: (message) => {
      if (!import.meta.client) return;
      // Extract text content from message parts
      const textContent = message.parts
        .filter(
          (p): p is { type: 'text'; content: string } => p.type === 'text',
        )
        .map((p) => p.content)
        .join('');
      // Extract tool calls if present
      const toolCalls: StoredToolCall[] = message.parts
        .filter(
          (
            p,
          ): p is {
            type: 'tool-call';
            toolCallId: string;
            toolName: string;
            args: Record<string, unknown>;
          } => p.type === 'tool-call',
        )
        .map((p) => ({
          id: p.toolCallId,
          name: p.toolName,
          args: JSON.stringify(p.args),
        }));
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
  } as ChatWithEngineStatus;
}
