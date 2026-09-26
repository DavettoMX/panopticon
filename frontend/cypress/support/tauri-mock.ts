// Helpers for controlling the Tauri mock layer from Cypress tests

interface TauriMockConfig {
  responses: Record<string, unknown>;
  errors: Record<string, string>;
  invocationLog: Array<{ cmd: string; args: unknown }>;
  emitEvent: (event: string, payload: unknown) => void;
}

declare global {
  interface Window {
    __TAURI_MOCK__: TauriMockConfig;
    __TAURI_MOCK_INITIAL__?: Record<string, unknown>;
  }
}

export function setMockResponse(cmd: string, response: unknown): void {
  cy.window().then((win) => {
    win.__TAURI_MOCK__.responses[cmd] = response;
  });
}

export function setMockError(cmd: string, errorMessage: string): void {
  cy.window().then((win) => {
    win.__TAURI_MOCK__.errors[cmd] = errorMessage;
  });
}

export function clearMockError(cmd: string): void {
  cy.window().then((win) => {
    delete win.__TAURI_MOCK__.errors[cmd];
  });
}

export function emitTauriEvent(event: string, payload: unknown): void {
  cy.window().then((win) => {
    win.__TAURI_MOCK__.emitEvent(event, payload);
  });
}

export function assertInvoked(
  cmd: string,
  argsMatcher?: (args: unknown) => boolean,
): void {
  cy.window().then((win) => {
    const calls = win.__TAURI_MOCK__.invocationLog.filter((c) => c.cmd === cmd);
    expect(calls.length, `Expected "${cmd}" to have been invoked`).to.be.greaterThan(0);
    if (argsMatcher) {
      const matched = calls.some((c) => argsMatcher(c.args));
      expect(matched, `Expected "${cmd}" to have been invoked with matching args`).to.be.true;
    }
  });
}

export function clearInvocationLog(): void {
  cy.window().then((win) => {
    win.__TAURI_MOCK__.invocationLog.length = 0;
  });
}
