import type { AdminFriendlyError } from '~/types/admin-mcp';
import type { OfetchLikeError } from '~/types/fetch';

function statusOf(error: Error | null): number | null {
  if (!error) return null;
  const e = error as OfetchLikeError;
  return e.statusCode ?? e.status ?? e.response?.status ?? null;
}

function isNetworkError(error: Error | null, status: number | null): boolean {
  if (!error) return false;
  if (status !== null) return false;
  const e = error as OfetchLikeError;
  if (e.response) return false;
  const message = error.message ?? '';
  return (
    /failed to fetch/i.test(message) ||
    /network ?error/i.test(message) ||
    /ERR_CONNECTION_REFUSED/i.test(message) ||
    /load failed/i.test(message)
  );
}

function statusMessageOf(error: Error | null): string | null {
  if (!error) return null;
  const e = error as OfetchLikeError;
  return e.statusMessage ?? e.response?.statusText ?? null;
}

function backendMessageOf(error: Error | null): string | null {
  if (!error) return null;
  const data = (error as OfetchLikeError).data;
  if (!data || typeof data !== 'object') return null;
  const obj = data as { error?: unknown; message?: unknown };
  const raw =
    typeof obj.error === 'string'
      ? obj.error
      : typeof obj.message === 'string'
        ? obj.message
        : null;
  if (!raw) return null;
  const trimmed = raw.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function requestPathOf(error: Error | null): string | null {
  if (!error) return null;
  const e = error as OfetchLikeError;
  if (typeof e.request === 'string') return e.request;
  return null;
}

function joinHint(
  parts: ReadonlyArray<string | null | undefined>,
): string | undefined {
  const present = parts.filter((p): p is string => Boolean(p));
  return present.length > 0 ? present.join(' · ') : undefined;
}

export function friendlyAdminError(error: Error | null): AdminFriendlyError {
  const status = statusOf(error);
  const statusMessage = statusMessageOf(error);
  const backendMessage = backendMessageOf(error);
  const requestPath = requestPathOf(error);
  const requestLabel = requestPath ? `Request: ${requestPath}` : null;

  if (isNetworkError(error, status)) {
    return {
      title: 'Admin server not reachable',
      body:
        'The browser could not open a TCP connection to the admin bind address. ' +
        'Either tileserver-rs is not running, the admin server was started without ' +
        '[server].admin_bind in config.toml, or a firewall is dropping the connection.',
      hint: joinHint([
        'Confirm `tileserver-rs --config <your-config.toml>` is running and ' +
          'that [server].admin_bind is set (e.g. "127.0.0.1:8081"). ' +
          'Without admin_bind, /__admin/* is not served at all.',
        requestLabel,
      ]),
    };
  }

  if (status === 404) {
    return {
      title: 'Admin endpoint not found',
      body:
        backendMessage ??
        'The admin server responded but does not expose this endpoint. ' +
          'This usually means the MCP OAuth store is not configured ' +
          '(no `[mcp.oauth]` block in config.toml), so the /__admin/oauth/* ' +
          'routes are not mounted.',
      hint: joinHint([
        'Add an `[mcp.oauth]` block to config.toml to enable MCP OAuth, ' +
          'then restart tileserver-rs. The MCP guide has the minimal config.',
        requestLabel,
      ]),
    };
  }
  if (status === 401 || status === 403) {
    return {
      title: 'Access denied',
      body:
        backendMessage ??
        'This page is restricted to the admin bind address. Make sure you ' +
          'opened it from a host where the admin server is reachable.',
      hint: requestLabel ?? undefined,
    };
  }
  if (status === 502 || status === 503 || status === 504) {
    return {
      title: 'Backend not reachable',
      body:
        `The dev proxy returned ${status}${statusMessage ? ` ${statusMessage}` : ''}. ` +
        'This means tileserver-rs is not listening on the admin bind address, ' +
        'or it is restarting.',
      hint: joinHint([
        'Confirm `tileserver-rs --config <your-config.toml>` is running and that ' +
          '[server].admin_bind in the config matches the dev-proxy target in nuxt.config.ts.',
        requestLabel,
      ]),
    };
  }
  if (status && status >= 500) {
    return {
      title: 'Admin endpoint failed',
      body:
        backendMessage ??
        `tileserver-rs returned ${status}${statusMessage ? ` ${statusMessage}` : ''} ` +
          'with no body. The server-side logs have the full trace.',
      hint: joinHint([
        'Check the tileserver-rs logs for the matching server-side error.',
        requestLabel,
      ]),
    };
  }
  return {
    title: 'Could not reach the admin server',
    body:
      backendMessage ??
      'The browser could not connect to the admin endpoint. Confirm that ' +
        'tileserver-rs is running and that the admin bind address is reachable from this host.',
    hint: requestLabel ?? undefined,
  };
}
