describe('Orchestrator Mode - Team Setup', () => {
  beforeEach(() => {
    cy.setupApp();
  });

  it('shows mode toggle tabs in sidebar', () => {
    cy.get('[data-cy=mode-tabs]').should('be.visible');
    cy.get('[data-cy=mode-tab-chat]').should('have.class', 'active');
    cy.get('[data-cy=mode-tab-orchestrator]').should('not.have.class', 'active');
  });

  it('switches to orchestrator mode and shows team setup', () => {
    cy.get('[data-cy=mode-tab-orchestrator]').click();
    cy.get('[data-cy=mode-tab-orchestrator]').should('have.class', 'active');
    cy.get('[data-cy=team-setup]').should('be.visible');
    cy.get('[data-cy=task-input]').should('be.visible');
  });

  it('shows all 5 agent role cards', () => {
    cy.get('[data-cy=mode-tab-orchestrator]').click();
    cy.get('[data-cy=agent-card-orchestrator]').should('exist');
    cy.get('[data-cy=agent-card-architect]').should('exist');
    cy.get('[data-cy=agent-card-auditor]').should('exist');
    cy.get('[data-cy=agent-card-constructor]').should('exist');
    cy.get('[data-cy=agent-card-judge]').should('exist');
  });

  it('opens model dropdown for an agent', () => {
    cy.get('[data-cy=mode-tab-orchestrator]').click();
    cy.get('[data-cy=agent-model-btn-orchestrator]').click();
    cy.get('[data-cy=agent-model-dropdown-orchestrator]').should('be.visible');
  });

  it('switches back to chat mode', () => {
    cy.get('[data-cy=mode-tab-orchestrator]').click();
    cy.get('[data-cy=team-setup]').should('be.visible');

    cy.get('[data-cy=mode-tab-chat]').click();
    cy.get('[data-cy=team-setup]').should('not.exist');
  });

  it('shows sessions list in sidebar when in orchestrator mode', () => {
    cy.get('[data-cy=mode-tab-orchestrator]').click();
    cy.get('[data-cy=sessions-label]').should('be.visible');
  });

  it('launch button is disabled without task and models', () => {
    cy.get('[data-cy=mode-tab-orchestrator]').click();
    cy.get('[data-cy=launch-team-btn]').should('be.disabled');
  });

  it('hides new chat button in orchestrator mode', () => {
    cy.get('[data-cy=new-chat-btn]').should('be.visible');
    cy.get('[data-cy=mode-tab-orchestrator]').click();
    cy.get('[data-cy=new-chat-btn]').should('not.exist');
  });
});
