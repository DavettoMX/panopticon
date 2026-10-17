<script lang="ts">
  import { ChevronDown, ChevronRight } from '@lucide/svelte';
  import { orchStore, ROLE_COLORS, type AgentRole } from '../orchestrator-store.svelte';
  import Markdown from '../Markdown.svelte';

  interface Props {
    role: AgentRole;
  }

  let { role }: Props = $props();

  const color = $derived(ROLE_COLORS[role]);
  const status = $derived(typeof orchStore.agentStatuses[role] === 'string' ? orchStore.agentStatuses[role] : 'idle');
  const streaming = $derived(typeof orchStore.agentStreaming[role] === 'string' ? orchStore.agentStreaming[role] : '');
  const isExpanded = $derived(orchStore.expandedAgents.has(role));
  const isSwapping = $derived(orchStore.swappingAgent === role);

  const statusDotColor = $derived(
    isSwapping ? '#60a5fa' :
    status === 'thinking' ? '#facc15' :
    status === 'done' ? '#4ade80' :
    status === 'error' ? '#f87171' :
    '#6b7280'
  );

  const displayStatus = $derived(
    isSwapping ? 'loading model...' :
    status
  );

  const messages = $derived(
    orchStore.executionLog.filter(m =>
      m.from_role === role || m.to_role === role
    )
  );

  function toggleExpand() {
    const next = new Set(orchStore.expandedAgents);
    if (isExpanded) {
      next.delete(role);
    } else {
      next.add(role);
    }
    orchStore.expandedAgents = next;
  }
</script>

<div class="agent-card" data-cy="agent-panel-{role}">
  <button class="agent-header" onclick={toggleExpand} data-cy="agent-toggle-{role}">
    <div class="agent-header-left">
      {#if isExpanded}
        <ChevronDown size={14} />
      {:else}
        <ChevronRight size={14} />
      {/if}
      <span class="agent-dot" style="background: {color}"></span>
      <span class="agent-name">{role.charAt(0).toUpperCase() + role.slice(1)}</span>
    </div>
    <div class="agent-header-right">
      <span class="status-dot" class:swapping-pulse={isSwapping} style="background: {statusDotColor}"></span>
      <span class="status-label">{displayStatus}</span>
      {#if messages.length > 0}
        <span class="msg-count">{messages.length}</span>
      {/if}
    </div>
  </button>

  {#if isExpanded}
    <div class="agent-body" data-cy="agent-body-{role}">
      {#each messages as msg}
        <div class="agent-msg" class:instruction={msg.message_type === 'instruction'} class:response={msg.message_type === 'response'}>
          <div class="msg-meta">
            <span class="msg-direction">
              {msg.message_type === 'instruction' ? 'Received from' : 'Sent to'}
              {msg.message_type === 'instruction' ? msg.from_role : msg.to_role}
            </span>
            <span class="msg-step">Step {msg.step_number}</span>
          </div>
          <div class="msg-content">
            <Markdown content={msg.content} />
          </div>
        </div>
      {/each}

      {#if streaming}
        <div class="agent-msg response streaming">
          <div class="msg-meta">
            <span class="msg-direction">Generating...</span>
          </div>
          <div class="msg-content">
            <Markdown content={streaming} />
            <span class="cursor-blink">|</span>
          </div>
        </div>
      {/if}

      {#if messages.length === 0 && !streaming}
        <div class="agent-empty">No activity yet</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .agent-card {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-card);
    overflow: hidden;
  }

  .agent-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 10px 12px;
    color: var(--text);
    cursor: pointer;
    transition: background 0.1s;
    text-align: left;
  }

  .agent-header:hover {
    background: var(--bg-hover);
  }

  .agent-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .agent-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .agent-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-bright);
  }

  .agent-header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .status-dot.swapping-pulse {
    animation: swap-pulse 1s ease-in-out infinite;
  }

  @keyframes swap-pulse {
    0%, 100% { opacity: 0.4; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.3); }
  }

  .status-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: capitalize;
  }

  .msg-count {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 8px;
    background: rgba(255,255,255,0.08);
    color: var(--text-muted);
  }

  .agent-body {
    border-top: 1px solid var(--border);
    padding: 8px;
    max-height: 400px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .agent-msg {
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 13px;
  }

  .agent-msg.instruction {
    background: rgba(167, 139, 250, 0.06);
    border-left: 3px solid rgba(167, 139, 250, 0.3);
  }

  .agent-msg.response {
    background: rgba(74, 222, 128, 0.06);
    border-left: 3px solid rgba(74, 222, 128, 0.3);
  }

  .msg-meta {
    display: flex;
    justify-content: space-between;
    margin-bottom: 4px;
  }

  .msg-direction {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--text-muted);
  }

  .msg-step {
    font-size: 10px;
    color: var(--text-muted);
  }

  .msg-content {
    color: var(--text);
    line-height: 1.5;
    word-wrap: break-word;
  }

  .msg-content :global(pre) {
    font-size: 12px;
  }

  .cursor-blink {
    animation: blink 0.8s step-end infinite;
    color: var(--text-bright);
  }

  @keyframes blink {
    50% { opacity: 0; }
  }

  .agent-empty {
    padding: 16px;
    text-align: center;
    font-size: 12px;
    color: var(--text-muted);
  }

  .streaming {
    border-left-color: rgba(250, 204, 21, 0.4);
    background: rgba(250, 204, 21, 0.04);
  }
</style>
