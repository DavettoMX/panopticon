describe('Chat Flow', () => {
  beforeEach(() => {
    cy.setupApp();
  });

  it('shows welcome screen when no conversation is active', () => {
    cy.get('[data-cy=welcome-screen]').should('be.visible');
    cy.get('[data-cy=message-input]').should(
      'have.attr',
      'placeholder',
      'Select a model to start chatting',
    );
  });

  it('shows suggestion buttons on welcome screen', () => {
    cy.get('[data-cy=suggestion-item]').should('have.length', 3);
    cy.get('[data-cy=suggestion-item]').first().should('contain', 'Help me study');
  });

  it('selects a model and shows ready state', () => {
    cy.selectModel('llama-3.2-1b-q4');
    cy.get('[data-cy=model-pill]').first().should('contain', 'Llama 3.2');
    cy.get('[data-cy=message-input]').should(
      'have.attr',
      'placeholder',
      'How can I help you today?',
    );
  });

  it('disables send button when no model is selected', () => {
    cy.get('[data-cy=message-input]').first().type('test');
    cy.get('[data-cy=send-btn]').first().should('be.disabled');
  });

  it('sends a message and receives streaming response', () => {
    const convId = 'conv-new-1';
    const newConv = {
      id: convId,
      title: 'New Conversation',
      model: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      deleted_at: null,
    };
    const assistantMsg = {
      id: 'msg-asst-1',
      conversation_id: convId,
      role: 'assistant',
      content: 'Hello! How can I help you?',
      model: 'llama-3.2-1b-q4.gguf',
      created_at: new Date().toISOString(),
    };

    cy.selectModel('llama-3.2-1b-q4');
    cy.get('[data-cy=message-input]').should('not.be.disabled');

    cy.mockTauriCommand('create_conversation', newConv);
    cy.mockTauriCommand('rename_conversation', null);

    // Make send_message hang so we can simulate streaming before it resolves
    cy.window().then((win) => {
      let resolve: (value: unknown) => void;
      win.__TAURI_MOCK__.responses['send_message'] = () =>
        new Promise((r) => { resolve = r; });
      // Store resolver on window so we can call it later
      (win as any).__TEST_RESOLVE_SEND__ = () => resolve(assistantMsg);
    });

    cy.get('[data-cy=message-input]').first().type('Hello there');
    cy.get('[data-cy=send-btn]').first().click();

    // User message appears optimistically
    cy.get('[data-cy=message-user]').should('contain', 'Hello there');

    // Simulate streaming tokens while send_message is still pending
    cy.simulateStreamingResponse(
      convId,
      ['Hello', '!', ' How', ' can', ' I', ' help', ' you', '?'],
      { id: 'msg-asst-1', content: 'Hello! How can I help you?' },
    );

    // Resolve send_message so the flow completes
    cy.window().then((win) => {
      (win as any).__TEST_RESOLVE_SEND__();
    });

    // Final assistant message visible
    cy.get('[data-cy=message-assistant]').should('contain', 'Hello! How can I help you?');

    // Stats visible after completion
    cy.get('[data-cy=generation-stats]').should('be.visible');

    // Verify IPC calls
    cy.assertTauriInvoked('create_conversation');
    cy.assertTauriInvoked('send_message', (args: any) => args.content === 'Hello there');
  });

  it('shows thinking indicator during reasoning', () => {
    const convId = 'conv-1';
    const assistantMsg = {
      id: 'msg-asst-new',
      conversation_id: convId,
      role: 'assistant',
      content: 'Thought-out response',
      model: 'llama-3.2-1b-q4.gguf',
      created_at: new Date().toISOString(),
    };

    cy.selectModel('llama-3.2-1b-q4');

    cy.fixture('messages.json').then((messages) => {
      cy.mockTauriCommand('get_messages', messages);
    });
    cy.get('[data-cy=conversation-link]').first().click();
    cy.get('[data-cy=messages-area]').should('be.visible');

    // Make send_message hang with a stored resolver
    cy.window().then((win) => {
      let resolve: (value: unknown) => void;
      win.__TAURI_MOCK__.responses['send_message'] = () =>
        new Promise((r) => { resolve = r; });
      (win as any).__TEST_RESOLVE_SEND__ = () => resolve(assistantMsg);
    });
    cy.mockTauriCommand('rename_conversation', null);

    cy.get('[data-cy=message-input]').type('Think about this');
    cy.get('[data-cy=send-btn]').click();

    // Emit reasoning token
    cy.emitTauriEvent('chat-token', {
      conversation_id: convId,
      content: 'reasoning...',
      is_reasoning: true,
    });

    cy.get('[data-cy=thinking-indicator]').should('contain', 'Thinking');

    // Resolve to clean up
    cy.window().then((win) => {
      (win as any).__TEST_RESOLVE_SEND__();
    });
  });

  it('handles generation error', () => {
    const convId = 'conv-1';

    cy.selectModel('llama-3.2-1b-q4');

    cy.fixture('messages.json').then((messages) => {
      cy.mockTauriCommand('get_messages', messages);
    });
    cy.get('[data-cy=conversation-link]').first().click();

    cy.window().then((win) => {
      let resolve: (value: unknown) => void;
      win.__TAURI_MOCK__.responses['send_message'] = () =>
        new Promise((r) => { resolve = r; });
      (win as any).__TEST_RESOLVE_SEND__ = () => resolve(null);
    });

    cy.get('[data-cy=message-input]').type('Error test');
    cy.get('[data-cy=send-btn]').click();

    cy.emitTauriEvent('chat-error', {
      conversation_id: convId,
      error: 'Model crashed unexpectedly',
    });

    cy.get('[data-cy=generation-error]').should('contain', 'Model crashed unexpectedly');

    // Resolve to clean up
    cy.window().then((win) => {
      (win as any).__TEST_RESOLVE_SEND__();
    });
  });

  it('shows stop button during generation and can stop', () => {
    const convId = 'conv-1';

    cy.selectModel('llama-3.2-1b-q4');

    cy.fixture('messages.json').then((messages) => {
      cy.mockTauriCommand('get_messages', messages);
    });
    cy.get('[data-cy=conversation-link]').first().click();

    // Make send_message hang with stored resolver
    cy.window().then((win) => {
      let resolve: (value: unknown) => void;
      win.__TAURI_MOCK__.responses['send_message'] = () =>
        new Promise((r) => { resolve = r; });
      (win as any).__TEST_RESOLVE_SEND__ = () => resolve(null);
    });

    cy.get('[data-cy=message-input]').type('Long running');
    cy.get('[data-cy=send-btn]').click();

    cy.get('[data-cy=stop-btn]').should('be.visible');
    cy.get('[data-cy=stop-btn]').click();
    cy.assertTauriInvoked('stop_generation');

    // Resolve to clean up
    cy.window().then((win) => {
      (win as any).__TEST_RESOLVE_SEND__();
    });
  });

  it('toggles thinking mode', () => {
    cy.get('[data-cy=thinking-toggle]').first().should('have.class', 'active');
    cy.get('[data-cy=thinking-toggle]').first().click();
    cy.get('[data-cy=thinking-toggle]').first().should('not.have.class', 'active');
  });
});
