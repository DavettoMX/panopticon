import 'cypress-real-events';
import {
  setMockResponse,
  emitTauriEvent,
  assertInvoked,
  clearInvocationLog,
  setMockError,
  clearMockError,
} from './tauri-mock';

declare global {
  namespace Cypress {
    interface Chainable {
      mockTauriCommand(cmd: string, response: unknown): Chainable<void>;
      mockTauriError(cmd: string, error: string): Chainable<void>;
      clearTauriError(cmd: string): Chainable<void>;
      emitTauriEvent(event: string, payload: unknown): Chainable<void>;
      assertTauriInvoked(
        cmd: string,
        argsMatcher?: (args: unknown) => boolean,
      ): Chainable<void>;
      clearTauriLog(): Chainable<void>;
      setupApp(): Chainable<void>;
      selectModel(modelName: string): Chainable<void>;
      sendChatMessage(text: string): Chainable<void>;
      simulateStreamingResponse(
        conversationId: string,
        tokens: string[],
        finalMessage: { id: string; content: string },
        stats?: object,
      ): Chainable<void>;
    }
  }
}

Cypress.Commands.add('mockTauriCommand', (cmd: string, response: unknown) => {
  setMockResponse(cmd, response);
});

Cypress.Commands.add('mockTauriError', (cmd: string, error: string) => {
  setMockError(cmd, error);
});

Cypress.Commands.add('clearTauriError', (cmd: string) => {
  clearMockError(cmd);
});

Cypress.Commands.add('emitTauriEvent', (event: string, payload: unknown) => {
  emitTauriEvent(event, payload);
});

Cypress.Commands.add(
  'assertTauriInvoked',
  (cmd: string, argsMatcher?: (args: unknown) => boolean) => {
    assertInvoked(cmd, argsMatcher);
  },
);

Cypress.Commands.add('clearTauriLog', () => {
  clearInvocationLog();
});

Cypress.Commands.add('setupApp', function () {
  cy.fixture('conversations.json').then((conversations) => {
    cy.fixture('models.json').then((models) => {
      cy.fixture('trash.json').then((trash) => {
        cy.visit('/', {
          onBeforeLoad(win) {
            (win as Window).__TAURI_MOCK_INITIAL__ = {
              list_conversations: conversations,
              list_models: models,
              list_trash: trash,
              get_messages: [],
              start_model: null,
              stop_generation: null,
              get_all_model_tags: {},
              get_model_details: {
                architecture: 'llama',
                quantization: 'Q4_K_M',
                parameter_count: 1000000000,
                layer_count: 22,
                embedding_length: 2048,
                context_length: 8192,
                head_count: 32,
                head_count_kv: 8,
                vocab_size: 32000,
                file_type: 14,
                tensor_count: 200,
              },
              get_gpu_info: {
                name: 'Mock GPU',
                vram_total_mb: 16384,
                vram_used_mb: 1024,
                vram_free_mb: 15360,
                compute_capability: '8.6',
                pcie_bandwidth_gbps: 32,
              },
              get_system_ram_mb: 32768,
              set_model_tags: null,
              create_conversation: null,
              delete_conversation: null,
              restore_conversation: null,
              permanent_delete_conversation: null,
              send_message: null,
              rename_conversation: null,
            };
          },
        });
      });
    });
  });

  cy.get('[data-cy=sidebar]').should('be.visible');
});

Cypress.Commands.add('selectModel', (modelName: string) => {
  cy.get('[data-cy=model-pill]').first().click();
  cy.get('[data-cy=model-dropdown]').should('be.visible');
  cy.get(`[data-cy=model-option][data-cy-model*="${modelName}"]`).click();
  // Config panel opens — wait for metadata to load, then click "Load Model"
  cy.get('[data-cy=load-model-btn]').should('exist').click({ force: true });
});

Cypress.Commands.add('sendChatMessage', (text: string) => {
  cy.get('[data-cy=message-input]').first().type(text);
  cy.get('[data-cy=send-btn]').first().click();
});

Cypress.Commands.add(
  'simulateStreamingResponse',
  (
    conversationId: string,
    tokens: string[],
    finalMessage: { id: string; content: string },
    stats?: object,
  ) => {
    for (const token of tokens) {
      cy.emitTauriEvent('chat-token', {
        conversation_id: conversationId,
        content: token,
        is_reasoning: false,
      });
    }

    cy.emitTauriEvent('chat-complete', {
      conversation_id: conversationId,
      message: {
        id: finalMessage.id,
        conversation_id: conversationId,
        role: 'assistant',
        content: finalMessage.content,
        model: 'llama-3.2-1b-q4.gguf',
        created_at: new Date().toISOString(),
      },
      stats: stats ?? {
        total_seconds: 2.5,
        thinking_seconds: 0,
        token_count: tokens.length,
        tokens_per_second: tokens.length / 2.5,
      },
    });
  },
);
