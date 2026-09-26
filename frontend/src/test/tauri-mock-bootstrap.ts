// Tauri IPC mock bootstrap — injected before main.ts when VITE_CYPRESS=true
// Sets up window.__TAURI_INTERNALS__ so @tauri-apps/api works without the native runtime

interface TauriMockConfig {
  responses: Record<string, unknown>;
  errors: Record<string, string>;
  invocationLog: Array<{ cmd: string; args: unknown }>;
  emitEvent: (event: string, payload: unknown) => void;
}

declare global {
  interface Window {
    __TAURI_INTERNALS__: unknown;
    __TAURI_MOCK__: TauriMockConfig;
    __TAURI_MOCK_INITIAL__?: Record<string, unknown>;
  }
}

const callbacks = new Map<number, (data: unknown) => unknown>();
const eventListeners = new Map<string, number[]>();
let nextCallbackId = 1;

function transformCallback(
  callback?: (data: unknown) => unknown,
  once = false,
): number {
  const id = nextCallbackId++;
  if (callback) {
    callbacks.set(id, (data: unknown) => {
      if (once) callbacks.delete(id);
      return callback(data);
    });
  }
  return id;
}

function runCallback(id: number, data: unknown) {
  const cb = callbacks.get(id);
  if (cb) cb(data);
}

const mockConfig: TauriMockConfig = {
  responses: {
    list_conversations: [],
    list_models: [],
    list_trash: [],
    get_messages: [],
    create_conversation: null,
    delete_conversation: null,
    restore_conversation: null,
    permanent_delete_conversation: null,
    start_model: null,
    send_message: null,
    rename_conversation: null,
    stop_generation: null,
  },
  errors: {},
  invocationLog: [],
  emitEvent(event: string, payload: unknown) {
    const listeners = eventListeners.get(event) || [];
    for (const handlerId of listeners) {
      runCallback(handlerId, { event, payload });
    }
  },
};

// Apply initial responses seeded by Cypress onBeforeLoad
if (window.__TAURI_MOCK_INITIAL__) {
  Object.assign(mockConfig.responses, window.__TAURI_MOCK_INITIAL__);
  delete window.__TAURI_MOCK_INITIAL__;
}

async function invoke(cmd: string, args?: unknown): Promise<unknown> {
  // Handle event plugin internal commands
  if (cmd === 'plugin:event|listen') {
    const { event, handler } = args as { event: string; handler: number };
    if (!eventListeners.has(event)) eventListeners.set(event, []);
    eventListeners.get(event)!.push(handler);
    return handler;
  }
  if (cmd === 'plugin:event|emit') {
    const { event, payload } = args as { event: string; payload: unknown };
    mockConfig.emitEvent(event, payload);
    return null;
  }
  if (cmd === 'plugin:event|unlisten') {
    const { event, eventId } = args as { event: string; eventId: number };
    const list = eventListeners.get(event);
    if (list) {
      const idx = list.indexOf(eventId);
      if (idx !== -1) list.splice(idx, 1);
    }
    return;
  }

  // Log invocation
  mockConfig.invocationLog.push({ cmd, args });

  // Check for programmed error
  if (mockConfig.errors[cmd]) {
    throw new Error(mockConfig.errors[cmd]);
  }

  // Return programmed response (supports function for dynamic responses)
  const response = mockConfig.responses[cmd];
  if (typeof response === 'function') {
    return (response as (a: unknown) => unknown)(args);
  }
  return response != null ? JSON.parse(JSON.stringify(response)) : null;
}

window.__TAURI_INTERNALS__ = {
  invoke,
  transformCallback,
  convertFileSrc: (src: string) => src,
  metadata: {
    currentWindow: { label: 'main' },
    currentWebview: { windowLabel: 'main', label: 'main' },
  },
  plugins: { path: { sep: '/', delimiter: ':' } },
};

window.__TAURI_MOCK__ = mockConfig;
