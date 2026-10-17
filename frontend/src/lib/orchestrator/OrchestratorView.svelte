<script lang="ts">
  import { Pause, Play, Square, Send, CheckCircle, AlertTriangle, Clock } from '@lucide/svelte';
  import {
    orchStore,
    AGENT_ROLES,
    pauseOrchestration,
    resumeOrchestration,
    stopOrchestration,
    injectUserMessage,
    type AgentRole,
  } from '../orchestrator-store.svelte';
  import TeamSetupPanel from './TeamSetupPanel.svelte';
  import AgentCard from './AgentCard.svelte';
  import FileProposalDialog from './FileProposalDialog.svelte';
  import Markdown from '../Markdown.svelte';

  let interventionText = $state('');

  const isActive = $derived(
    orchStore.status === 'running' ||
    orchStore.status === 'paused' ||
    orchStore.status === 'settingup'
  );

  const isDone = $derived(
    orchStore.status === 'complete' || orchStore.status === 'error'
  );

  async function handleIntervene() {
    if (!interventionText.trim()) return;
    await injectUserMessage(interventionText.trim());
    interventionText = '';
  }

  function handleInterventionKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleIntervene();
    }
  }

  const statusLabel = $derived(
    orchStore.status === 'idle' ? 'Ready' :
    orchStore.status === 'settingup' ? 'Starting servers...' :
    orchStore.status === 'running' ? 'Running' :
    orchStore.status === 'paused' ? 'Paused at checkpoint' :
    orchStore.status === 'complete' ? 'Complete' :
    orchStore.status === 'error' ? 'Error' : ''
  );


</script>

<div class="orchestrator-view" data-cy="orchestrator-view">
  {#if !isActive && !isDone}
    <TeamSetupPanel />
  {:else}
    <!-- Status Bar -->
    <div class="status-bar" data-cy="orch-status-bar">
      <div class="status-left">
        <span class="status-badge" class:running={orchStore.status === 'running'} class:paused={orchStore.status === 'paused'} class:complete={orchStore.status === 'complete'} class:error={orchStore.status === 'error'} class:settingup={orchStore.status === 'settingup'}>
          {#if orchStore.status === 'complete'}
            <CheckCircle size={14} />
          {:else if orchStore.status === 'error'}
            <AlertTriangle size={14} />
          {:else if orchStore.status === 'settingup'}
            <Clock size={14} />
          {:else}
            <span class="status-pulse"></span>
          {/if}
          {statusLabel}
        </span>
        <span class="step-counter">Step {orchStore.currentStep}</span>
      </div>
      <div class="status-right">
        {#if orchStore.status === 'running'}
          <button class="control-btn" onclick={pauseOrchestration} data-cy="pause-btn" title="Pause">
            <Pause size={16} />
          </button>
        {/if}
        {#if orchStore.status === 'paused'}
          <button class="control-btn resume" onclick={resumeOrchestration} data-cy="resume-btn" title="Resume">
            <Play size={16} />
          </button>
        {/if}
        {#if isActive}
          <button class="control-btn stop" onclick={stopOrchestration} data-cy="stop-btn" title="Stop">
            <Square size={16} />
          </button>
        {/if}
      </div>
    </div>

    {#if orchStore.error}
      <div class="error-banner" data-cy="orch-error">
        <AlertTriangle size={14} />
        {orchStore.error}
      </div>
    {/if}

    <!-- Main Content: Timeline + Agents -->
    <div class="execution-content">
      <!-- Timeline Column -->
      <div class="timeline-column" data-cy="orch-timeline">
        <h3 class="column-title">Orchestrator Timeline</h3>

        {#if orchStore.executionLog.length === 0 && orchStore.status !== 'idle'}
          <div class="timeline-empty">
            {#if orchStore.status === 'settingup'}
              Starting agent servers...
            {:else}
              Waiting for orchestrator...
            {/if}
          </div>
        {/if}

        {#each orchStore.executionLog as entry (entry.id)}
          <div class="timeline-entry" class:checkpoint={entry.message_type === 'checkpoint'} class:complete={entry.message_type === 'complete'} class:response={entry.message_type === 'response'} data-cy="timeline-entry">
            <div class="timeline-entry-header">
              <span class="timeline-step">Step {entry.step_number}</span>
              {#if entry.message_type === 'checkpoint'}
                <span class="timeline-type checkpoint-badge">Checkpoint</span>
              {:else if entry.message_type === 'complete'}
                <span class="timeline-type complete-badge">Complete</span>
              {:else if entry.message_type === 'tool_call'}
                <span class="timeline-type tool-badge">{entry.from_role} tool call</span>
              {:else if entry.message_type === 'file_write'}
                <span class="timeline-type file-badge">Files Written</span>
              {:else if entry.message_type === 'response'}
                <span class="timeline-type response-badge">{entry.from_role} responded</span>
              {:else}
                <span class="timeline-type delegate-badge">{entry.from_role} &rarr; {entry.to_role}</span>
              {/if}
            </div>
            <div class="timeline-content">
              <Markdown content={entry.content} />
            </div>
          </div>
        {/each}

        <!-- Checkpoint Intervention -->
        {#if orchStore.status === 'paused' && orchStore.checkpointSummary}
          <div class="checkpoint-panel" data-cy="checkpoint-panel">
            <div class="checkpoint-summary">
              <CheckCircle size={14} />
              <span>{orchStore.checkpointSummary}</span>
            </div>
            <div class="intervention-input-wrapper">
              <textarea
                class="intervention-input"
                placeholder="Add feedback or press Resume to continue..."
                bind:value={interventionText}
                onkeydown={handleInterventionKeydown}
                rows="2"
                data-cy="intervention-input"
              ></textarea>
              <button
                class="intervention-send"
                onclick={handleIntervene}
                disabled={!interventionText.trim()}
                data-cy="intervention-send-btn"
              >
                <Send size={14} />
              </button>
            </div>
          </div>
        {/if}
      </div>

      <!-- Agent Panels Column -->
      <div class="agents-column" data-cy="agent-panels">
        <h3 class="column-title">Agent Activity</h3>
        {#each AGENT_ROLES as roleInfo}
          <AgentCard role={roleInfo.value} />
        {/each}
      </div>
    </div>
  {/if}

  <FileProposalDialog />
</div>

<style>
  .orchestrator-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 20px;
    background: var(--bg-sidebar);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .status-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
  }

  .status-badge.running {
    background: rgba(74, 222, 128, 0.1);
    color: #4ade80;
  }

  .status-badge.paused {
    background: rgba(250, 204, 21, 0.1);
    color: #facc15;
  }

  .status-badge.complete {
    background: rgba(96, 165, 250, 0.1);
    color: #60a5fa;
  }

  .status-badge.error {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .status-badge.settingup {
    background: rgba(167, 139, 250, 0.1);
    color: #a78bfa;
  }

  .status-pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }

  .step-counter {
    font-size: 12px;
    color: var(--text-muted);
  }

  .status-right {
    display: flex;
    gap: 6px;
  }

  .control-btn {
    padding: 6px;
    border-radius: 6px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    transition: all 0.15s;
  }

  .control-btn:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .control-btn.resume:hover {
    color: #4ade80;
  }

  .control-btn.stop:hover {
    color: #f87171;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    background: rgba(248, 113, 113, 0.08);
    border-bottom: 1px solid rgba(248, 113, 113, 0.2);
    color: #f87171;
    font-size: 13px;
    flex-shrink: 0;
  }

  .execution-content {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .timeline-column {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    border-right: 1px solid var(--border);
  }

  .agents-column {
    width: 380px;
    min-width: 320px;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .column-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: 12px;
  }

  .timeline-empty {
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
    padding: 40px 20px;
  }

  .timeline-entry {
    padding: 12px;
    border-radius: 8px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    margin-bottom: 8px;
  }

  .timeline-entry.checkpoint {
    border-color: rgba(250, 204, 21, 0.3);
  }

  .timeline-entry.complete {
    border-color: rgba(96, 165, 250, 0.3);
  }

  .timeline-entry-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .timeline-step {
    font-size: 11px;
    color: var(--text-muted);
  }

  .timeline-type {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .delegate-badge {
    background: rgba(167, 139, 250, 0.12);
    color: #a78bfa;
  }

  .checkpoint-badge {
    background: rgba(250, 204, 21, 0.12);
    color: #facc15;
  }

  .complete-badge {
    background: rgba(96, 165, 250, 0.12);
    color: #60a5fa;
  }

  .response-badge {
    background: rgba(74, 222, 128, 0.12);
    color: #4ade80;
    text-transform: capitalize;
  }

  .tool-badge {
    background: rgba(251, 146, 60, 0.12);
    color: #fb923c;
    text-transform: capitalize;
  }

  .file-badge {
    background: rgba(96, 165, 250, 0.12);
    color: #60a5fa;
  }

  .timeline-entry.response {
    border-color: rgba(74, 222, 128, 0.2);
  }

  .timeline-content {
    font-size: 13px;
    color: var(--text);
    line-height: 1.5;
    word-wrap: break-word;
  }

  .checkpoint-panel {
    padding: 14px;
    border-radius: 8px;
    background: rgba(250, 204, 21, 0.05);
    border: 1px solid rgba(250, 204, 21, 0.2);
    margin-bottom: 8px;
  }

  .checkpoint-summary {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 13px;
    color: #facc15;
    margin-bottom: 10px;
  }

  .intervention-input-wrapper {
    display: flex;
    gap: 8px;
  }

  .intervention-input {
    flex: 1;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-bright);
    padding: 8px 10px;
    font-size: 13px;
    font-family: var(--sans);
    resize: none;
  }

  .intervention-input:focus {
    outline: none;
    border-color: var(--primary-light);
  }

  .intervention-input::placeholder {
    color: var(--text-muted);
  }

  .intervention-send {
    padding: 8px;
    border-radius: 6px;
    background: var(--primary);
    color: var(--text-bright);
    display: flex;
    align-items: center;
    transition: background 0.15s;
    align-self: flex-end;
  }

  .intervention-send:hover:not(:disabled) {
    background: var(--primary-light);
  }

  .intervention-send:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
