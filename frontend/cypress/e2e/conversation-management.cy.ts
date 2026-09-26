describe('Conversation Management', () => {
  beforeEach(() => {
    cy.setupApp();
  });

  it('displays conversations in sidebar', () => {
    cy.get('[data-cy=conversation-item]').should('have.length', 2);
    cy.get('[data-cy=conversation-title]').first().should('contain', 'First conversation');
  });

  it('creates a new conversation', () => {
    const newConv = {
      id: 'conv-new',
      title: 'New Conversation',
      model: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      deleted_at: null,
    };
    cy.mockTauriCommand('create_conversation', newConv);

    cy.get('[data-cy=new-chat-btn]').click();

    cy.assertTauriInvoked('create_conversation');
    cy.get('[data-cy=conversation-title]').first().should('contain', 'New Conversation');
  });

  it('loads messages when clicking a conversation', () => {
    cy.fixture('messages.json').then((messages) => {
      cy.mockTauriCommand('get_messages', messages);
    });

    cy.get('[data-cy=conversation-link]').first().click();

    cy.assertTauriInvoked('get_messages', (args: any) => args.conversationId === 'conv-1');
    cy.get('[data-cy=messages-area]').should('be.visible');
    cy.get('[data-cy=message-user]').should('contain', 'Hello, how are you?');
    cy.get('[data-cy=message-assistant]').should('contain', "I'm doing well");
  });

  it('soft-deletes a conversation', () => {
    cy.mockTauriCommand('delete_conversation', null);

    cy.get('[data-cy=conversation-item]').first()
      .find('[data-cy=delete-conversation-btn]').click({ force: true });

    cy.assertTauriInvoked('delete_conversation', (args: any) => args.id === 'conv-1');
    cy.get('[data-cy=conversation-title]').should('not.contain', 'First conversation');
  });

  it('navigates to trash view and shows deleted conversations', () => {
    cy.get('[data-cy=trash-toggle]').click();
    cy.get('[data-cy=trash-label]').should('be.visible');
    cy.get('[data-cy=trash-item]').should('have.length', 1);
    cy.get('[data-cy=trash-item]').should('contain', 'Deleted conversation');
  });

  it('shows trash count badge', () => {
    cy.get('[data-cy=trash-count]').should('contain', '1');
  });

  it('restores a conversation from trash', () => {
    cy.mockTauriCommand('restore_conversation', null);

    cy.get('[data-cy=trash-toggle]').click();
    cy.get('[data-cy=restore-btn]').first().click();

    cy.assertTauriInvoked('restore_conversation', (args: any) => args.id === 'conv-deleted-1');
    cy.get('[data-cy=trash-item]').should('have.length', 0);
    cy.get('[data-cy=trash-empty]').should('be.visible');
  });

  it('permanently deletes a conversation from trash', () => {
    cy.mockTauriCommand('permanent_delete_conversation', null);

    cy.get('[data-cy=trash-toggle]').click();
    cy.get('[data-cy=permanent-delete-btn]').first().click();

    cy.assertTauriInvoked('permanent_delete_conversation', (args: any) => args.id === 'conv-deleted-1');
    cy.get('[data-cy=trash-item]').should('have.length', 0);
  });

  it('navigates back from trash to conversations', () => {
    cy.get('[data-cy=trash-toggle]').click();
    cy.get('[data-cy=trash-toggle]').should('contain', 'Back to chats');
    cy.get('[data-cy=trash-toggle]').click();
    cy.get('[data-cy=conversations-label]').should('be.visible');
  });

  it('returns to welcome screen when deleting the active conversation', () => {
    cy.fixture('messages.json').then((messages) => {
      cy.mockTauriCommand('get_messages', messages);
    });
    cy.get('[data-cy=conversation-link]').first().click();
    cy.get('[data-cy=messages-area]').should('be.visible');

    cy.mockTauriCommand('delete_conversation', null);
    cy.get('[data-cy=conversation-item]').first()
      .find('[data-cy=delete-conversation-btn]').click({ force: true });

    cy.get('[data-cy=welcome-screen]').should('be.visible');
  });
});
