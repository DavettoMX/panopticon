<script lang="ts">
  import { AlertTriangle } from '@lucide/svelte';
  import { orchStore, ROLE_COLORS, type AgentRole } from '../orchestrator-store.svelte';

  const estimate = $derived(orchStore.vramEstimate);
  const totalBar = $derived(estimate ? Math.max(estimate.total_gpu_mb, estimate.gpu_available_mb) : 0);

  const orchAgent = $derived(estimate?.agents.find(a => a.role === 'orchestrator'));
  const otherAgents = $derived(estimate?.agents.filter(a => a.role !== 'orchestrator') ?? []);
  const maxOtherAgent = $derived(
    otherAgents.length > 0
      ? otherAgents.reduce((max, a) => a.gpu_mem_mb > max.gpu_mem_mb ? a : max, otherAgents[0])
      : null
  );
</script>

{#if estimate}
  <div class="vram-preview" data-cy="vram-preview">
    <div class="vram-header">
      <span class="vram-title">GPU Memory (swap-on-demand)</span>
      <span class="vram-summary" class:overflow={estimate.will_overflow}>
        {(estimate.total_gpu_mb / 1024).toFixed(1)} GB / {(estimate.gpu_available_mb / 1024).toFixed(1)} GB
      </span>
    </div>

    <div class="vram-bar-container">
      <div class="vram-bar-bg">
        {#if orchAgent}
          {@const pct = totalBar > 0 ? (orchAgent.gpu_mem_mb / totalBar) * 100 : 0}
          <div
            class="vram-bar-segment"
            style="width: {pct}%; background: {ROLE_COLORS['orchestrator']}"
            title="Orchestrator (fixed): {(orchAgent.gpu_mem_mb / 1024).toFixed(1)} GB"
            data-cy="vram-segment-orchestrator"
          ></div>
        {/if}
        {#if maxOtherAgent}
          {@const pct = totalBar > 0 ? (maxOtherAgent.gpu_mem_mb / totalBar) * 100 : 0}
          <div
            class="vram-bar-segment"
            style="width: {pct}%; background: {ROLE_COLORS[maxOtherAgent.role as AgentRole] || '#666'}; opacity: 0.6"
            title="Rotating slot (max): {(maxOtherAgent.gpu_mem_mb / 1024).toFixed(1)} GB"
            data-cy="vram-segment-rotating"
          ></div>
        {/if}
      </div>
      {#if estimate.gpu_available_mb > 0 && totalBar > 0}
        <div
          class="vram-limit-marker"
          style="left: {(estimate.gpu_available_mb / totalBar) * 100}%"
        ></div>
      {/if}
    </div>

    <div class="vram-breakdown">
      {#if orchAgent}
        <div class="vram-agent-row" data-cy="vram-agent-orchestrator">
          <span class="vram-dot" style="background: {ROLE_COLORS['orchestrator']}"></span>
          <span class="vram-role">orchestrator</span>
          <span class="vram-model">{orchAgent.model_filename}</span>
          <span class="vram-value">{(orchAgent.gpu_mem_mb / 1024).toFixed(1)} GB</span>
          <span class="vram-slot-badge fixed">fixed</span>
          {#if orchAgent.needs_cpu_offload}
            <span class="vram-offload-badge">CPU offload</span>
          {/if}
        </div>
      {/if}
      {#each otherAgents as agent}
        <div class="vram-agent-row" data-cy="vram-agent-{agent.role}">
          <span class="vram-dot" style="background: {ROLE_COLORS[agent.role as AgentRole] || '#666'}"></span>
          <span class="vram-role">{agent.role}</span>
          <span class="vram-model">{agent.model_filename}</span>
          <span class="vram-value">{(agent.gpu_mem_mb / 1024).toFixed(1)} GB</span>
          <span class="vram-slot-badge rotating">rotating</span>
          {#if agent.needs_cpu_offload}
            <span class="vram-offload-badge">CPU offload</span>
          {/if}
        </div>
      {/each}
    </div>

    {#if estimate.will_overflow}
      <div class="vram-warning" data-cy="vram-warning">
        <AlertTriangle size={14} />
        <span>VRAM overflow by {(estimate.overflow_mb / 1024).toFixed(1)} GB. Reduce GPU layers or use smaller models.</span>
      </div>
    {/if}

    {#if estimate.total_cpu_mb > 0}
      <div class="vram-cpu-info">
        <span>System RAM: {(estimate.total_cpu_mb / 1024).toFixed(1)} GB (CPU offload)</span>
      </div>
    {/if}
  </div>
{/if}

<style>
  .vram-preview {
    padding: 16px;
    background: var(--bg-card);
    border-radius: 10px;
    border: 1px solid var(--border);
  }

  .vram-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }

  .vram-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .vram-summary {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-bright);
  }

  .vram-summary.overflow {
    color: #f87171;
  }

  .vram-bar-container {
    position: relative;
    margin-bottom: 12px;
  }

  .vram-bar-bg {
    display: flex;
    height: 12px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    overflow: hidden;
  }

  .vram-bar-segment {
    height: 100%;
    min-width: 2px;
    opacity: 0.85;
    transition: width 0.3s ease;
  }

  .vram-limit-marker {
    position: absolute;
    top: -2px;
    bottom: -2px;
    width: 2px;
    background: var(--text-bright);
    border-radius: 1px;
  }

  .vram-breakdown {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .vram-agent-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .vram-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .vram-role {
    color: var(--text-bright);
    font-weight: 500;
    min-width: 80px;
    text-transform: capitalize;
  }

  .vram-model {
    color: var(--text-muted);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .vram-value {
    color: var(--text);
    flex-shrink: 0;
  }

  .vram-slot-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .vram-slot-badge.fixed {
    background: rgba(167, 139, 250, 0.15);
    color: #a78bfa;
  }

  .vram-slot-badge.rotating {
    background: rgba(96, 165, 250, 0.15);
    color: #60a5fa;
  }

  .vram-offload-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(251, 146, 60, 0.15);
    color: #fb923c;
    flex-shrink: 0;
  }

  .vram-warning {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    background: rgba(248, 113, 113, 0.1);
    border: 1px solid rgba(248, 113, 113, 0.2);
    color: #f87171;
    font-size: 12px;
    margin-top: 8px;
  }

  .vram-cpu-info {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 6px;
  }
</style>
