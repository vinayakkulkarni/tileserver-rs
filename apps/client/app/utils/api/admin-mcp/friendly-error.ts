import type { AdminFriendlyError } from '~/types/admin-mcp';

function statusOf(error: Error | null): number | null {
  if (!error) return null;
  const e = error as {
    statusCode?: number;
    status?: number;
    response?: { status?: number };
  };
  return e.statusCode ?? e.status ?? e.response?.status ?? null;
}

export function friendlyAdminError(error: Error | null): AdminFriendlyError {
  const status = statusOf(error);

  if (status === 404) {
    return {
      title: 'Admin server not available',
      body:
        'The MCP admin endpoints did not respond. This usually means the ' +
        'admin server is disabled, or OAuth is not configured for MCP.',
      hint:
        'Enable [server].admin_bind and [mcp.oauth] in your config.toml, ' +
        'then restart tileserver-rs.',
    };
  }
  if (status === 401 || status === 403) {
    return {
      title: 'Access denied',
      body:
        'This page is restricted to the admin bind address. Make sure you ' +
        'opened it from the host where the admin server is reachable.',
    };
  }
  if (status && status >= 500) {
    return {
      title: 'Something went wrong',
      body:
        'The admin endpoint returned an unexpected error. Try again in a ' +
        'moment.',
      hint: 'If the problem persists, check the tileserver-rs logs.',
    };
  }
  return {
    title: 'Could not reach the admin server',
    body:
      'The browser could not connect to the admin endpoint. Confirm that ' +
      'tileserver-rs is running.',
  };
}
