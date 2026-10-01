# Panopticon

Tauri v2 desktop app — local LLM chat UI powered by llama.cpp.
Frontend: Svelte 5 + TypeScript (Vite). Backend: Rust (Tauri v2).

## Project Structure

```
frontend/               Svelte/Vite UI
  src/lib/              Components + stores
  src/lib/stores.svelte.ts  Shared reactive state (Svelte 5 runes)
  src/test/             Tauri IPC mock layer for Cypress
  cypress/              E2E tests, fixtures, support commands
src-tauri/              Rust backend (Tauri v2)
  src/commands/         Tauri command handlers (chat, conversations, models, server, settings)
  src/db.rs             SQLite schema + CRUD
  src/gguf.rs           GGUF binary metadata parser
  src/gpu.rs            GPU detection (nvidia-smi)
  src/llama.rs          llama-server process lifecycle
```

## Running

```bash
npm run tauri dev          # Full app (from root)
cd frontend
npm run dev:test           # Vite with Tauri mocks (for Cypress)
npm run cy:open            # Cypress interactive (needs dev:test running)
npm run cy:run             # Cypress headless
```

## Frontend Guidelines

- Use the **frontend-design** skill when creating or redesigning UI components
- Svelte 5 runes: `$state`, `$derived`, `$effect` — module-level reactive state only in `.svelte.ts` files
- Use `data-cy` attributes on all interactive elements for test selectors
- Component styles are scoped via `<style>` blocks — avoid `color-mix()` and other CSS functions that break `vite-plugin-svelte`'s virtual CSS module loader; use inline styles for dynamic colors instead
- `svelte.config.js` uses `css: 'injected'` to bypass virtual CSS module issues with Vite 8

## Cypress E2E Testing

### Skills

Use these skills when working with Cypress tests:

- **cypress-author** — Creating, updating, or fixing tests. Use even when "Cypress" isn't mentioned (e.g., "add tests for this component")
- **cypress-explain** — Explaining how a test works, reviewing or critiquing tests without code changes
- **cypress-docs** — Looking up Cypress API, behavior, or best practices from official docs

### Architecture

The app runs without the native Tauri runtime in test mode. A mock layer (`frontend/src/test/tauri-mock-bootstrap.ts`) is injected via Vite plugin when `VITE_CYPRESS=true`, intercepting all `@tauri-apps/api` IPC calls.

- **Mock bootstrap** (`tauri-mock-bootstrap.ts`): Sets up `window.__TAURI_INTERNALS__` with a mock `invoke()` that returns pre-configured responses. Unmocked commands return `null`.
- **Initial data**: Seeded via `window.__TAURI_MOCK_INITIAL__` in Cypress `onBeforeLoad` (see `setupApp` command)
- **Runtime control**: Tests manipulate mocks via `window.__TAURI_MOCK__` (responses, errors, emitEvent, invocationLog)

### Custom Commands (`cypress/support/commands.ts`)

| Command | Purpose |
|---------|---------|
| `cy.setupApp()` | Visit app with all fixtures loaded as mock data |
| `cy.selectModel(name)` | Open dropdown, pick model, click through config panel |
| `cy.sendChatMessage(text)` | Type and send a message |
| `cy.simulateStreamingResponse(convId, tokens, finalMsg)` | Emit streaming events |
| `cy.mockTauriCommand(cmd, response)` | Override a mock response at runtime |
| `cy.mockTauriError(cmd, error)` | Make a command throw |
| `cy.emitTauriEvent(event, payload)` | Simulate a Tauri event |
| `cy.assertTauriInvoked(cmd, argsMatcher?)` | Assert a command was called |

### Writing Tests

1. **Always add new Tauri commands to mocks**: When adding a new `invoke()` call in the frontend, add a default response in both `tauri-mock-bootstrap.ts` (defaults) and `setupApp` in `commands.ts`
2. **Use `data-cy` selectors**: Never query by class name or tag — use `[data-cy=...]` attributes
3. **Deferred promises for async flows**: For commands like `send_message` that need to hang while streaming is simulated, use the deferred resolver pattern:
   ```ts
   cy.window().then((win) => {
     let resolve: (value: unknown) => void;
     win.__TAURI_MOCK__.responses['send_message'] = () =>
       new Promise((r) => { resolve = r; });
     (win as any).__TEST_RESOLVE_SEND__ = () => resolve(result);
   });
   ```
4. **Force-click overlayed elements**: The ModelConfigPanel overlay can block visibility checks — use `click({ force: true })` when clicking through overlays
5. **Fixtures** are in `cypress/fixtures/` — update them when backend types change (e.g., adding fields to `ModelInfo`)

## Rust Backend Guidelines

- `std::sync::Mutex` guards CANNOT be held across `.await` — take value out, drop guard, then await
- Tauri permissions: use `core:event:default` (NOT `event:default`)
- DB migrations go in `run_migrations()` in `db.rs` — check column existence before altering

## Key Architecture Decisions

- Models discovered by scanning configurable directory for `.gguf` files
- GGUF metadata parsed from binary headers (no external tools needed)
- Model categorization: SLM (<10B params) vs LLM (>=10B) based on `parameter_count` from GGUF
- Task tags (coding, chat, vision, etc.) stored in `model_tags` SQLite table
- llama-server spawned as subprocess, communicates via OpenAI-compatible REST API on port 8080
- Streaming via Tauri events (`chat-token`, `chat-complete`, `chat-error`)
