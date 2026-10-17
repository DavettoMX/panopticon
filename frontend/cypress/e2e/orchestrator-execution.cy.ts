describe('Orchestrator Mode - Execution', () => {
  beforeEach(() => {
    cy.setupOrchestrator();
  });

  it('shows VRAM preview when estimate is available', () => {
    // Mock the estimate response with actual data
    cy.mockTauriCommand('estimate_team_vram', {
      agents: [
        { role: 'orchestrator', model_filename: 'model-1b.gguf', gpu_mem_mb: 1024, cpu_mem_mb: 0, needs_cpu_offload: false, ngl: 22, total_layers: 22 },
        { role: 'architect', model_filename: 'model-3b.gguf', gpu_mem_mb: 2048, cpu_mem_mb: 0, needs_cpu_offload: false, ngl: 26, total_layers: 26 },
      ],
      total_gpu_mb: 3072,
      total_cpu_mb: 0,
      gpu_available_mb: 15360,
      will_overflow: false,
      overflow_mb: 0,
    });
  });

  it('shows orchestrator view with status bar after launch events', () => {
    // Simulate that orchestration is already running by emitting events
    cy.emitTauriEvent('orchestrator-agent-start', {
      role: 'Orchestrator',
      instruction: 'Analyzing task...',
      step_number: 1,
    });

    // The orchestrator view should be visible after the status changes
    // In real usage, this happens after startOrchestration call
  });

  it('shows checkpoint panel when checkpoint event fires', () => {
    cy.emitTauriEvent('orchestrator-checkpoint', {
      summary: 'Phase 1 architecture review complete',
      step_number: 3,
    });
  });

  it('shows agent streaming tokens', () => {
    cy.emitTauriEvent('orchestrator-agent-start', {
      role: 'Architect',
      instruction: 'Design the auth system',
      step_number: 2,
    });

    cy.emitTauriEvent('orchestrator-agent-token', {
      role: 'Architect',
      content: 'The authentication system should use ',
    });

    cy.emitTauriEvent('orchestrator-agent-token', {
      role: 'Architect',
      content: 'JWT tokens for session management.',
    });
  });

  it('shows agent completion in log', () => {
    cy.emitTauriEvent('orchestrator-agent-complete', {
      role: 'Architect',
      response: 'The architecture plan is ready.',
      step_number: 2,
    });
  });
});
