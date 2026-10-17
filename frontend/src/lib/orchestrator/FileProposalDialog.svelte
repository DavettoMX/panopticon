<script lang="ts">
  import { FileText, FilePlus, Check, X, ChevronDown, ChevronRight } from '@lucide/svelte';
  import {
    orchStore,
    approveFileWrites,
    denyFileWrites,
  } from '../orchestrator-store.svelte';

  let expandedFiles = $state(new Set<number>());

  function toggleFile(index: number) {
    const next = new Set(expandedFiles);
    if (next.has(index)) {
      next.delete(index);
    } else {
      next.add(index);
    }
    expandedFiles = next;
  }

  const files = $derived(orchStore.pendingFileProposal?.files ?? []);
</script>

{#if orchStore.pendingFileProposal}
  <div class="overlay" data-cy="file-proposal-overlay">
    <div class="dialog" data-cy="file-proposal-dialog">
      <div class="dialog-header">
        <h3>Constructor wants to write files</h3>
        <p class="dialog-subtitle">
          {files.length} file{files.length !== 1 ? 's' : ''} will be written to the working directory.
          Review and approve or deny.
        </p>
      </div>

      <div class="file-list">
        {#each files as file, i}
          <div class="file-item" data-cy="file-proposal-item">
            <button class="file-header" onclick={() => toggleFile(i)}>
              <span class="file-icon">
                {#if file.is_new}
                  <FilePlus size={14} />
                {:else}
                  <FileText size={14} />
                {/if}
              </span>
              <span class="file-path">{file.path}</span>
              <span class="file-badge" class:new={file.is_new} class:modified={!file.is_new}>
                {file.is_new ? 'new' : 'modify'}
              </span>
              <span class="file-expand">
                {#if expandedFiles.has(i)}
                  <ChevronDown size={14} />
                {:else}
                  <ChevronRight size={14} />
                {/if}
              </span>
            </button>
            {#if expandedFiles.has(i)}
              <div class="file-content">
                <pre><code>{file.content}</code></pre>
              </div>
            {/if}
          </div>
        {/each}
      </div>

      <div class="dialog-actions">
        <button class="btn deny" onclick={denyFileWrites} data-cy="deny-files-btn">
          <X size={16} />
          Deny
        </button>
        <button class="btn approve" onclick={approveFileWrites} data-cy="approve-files-btn">
          <Check size={16} />
          Approve & Write
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }

  .dialog {
    background: var(--bg-sidebar);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 90%;
    max-width: 700px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  }

  .dialog-header {
    padding: 20px 24px 12px;
    border-bottom: 1px solid var(--border);
  }

  .dialog-header h3 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-bright);
    margin: 0 0 4px;
  }

  .dialog-subtitle {
    font-size: 13px;
    color: var(--text-muted);
    margin: 0;
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .file-item {
    border: 1px solid var(--border);
    border-radius: 8px;
    margin-bottom: 8px;
    overflow: hidden;
  }

  .file-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 10px 12px;
    text-align: left;
    color: var(--text);
    transition: background 0.1s;
  }

  .file-header:hover {
    background: var(--bg-hover);
  }

  .file-icon {
    display: flex;
    align-items: center;
    color: var(--text-muted);
  }

  .file-path {
    flex: 1;
    font-family: var(--mono, monospace);
    font-size: 13px;
    color: var(--text-bright);
  }

  .file-badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .file-badge.new {
    background: rgba(74, 222, 128, 0.12);
    color: #4ade80;
  }

  .file-badge.modified {
    background: rgba(250, 204, 21, 0.12);
    color: #facc15;
  }

  .file-expand {
    display: flex;
    align-items: center;
    color: var(--text-muted);
  }

  .file-content {
    border-top: 1px solid var(--border);
    max-height: 300px;
    overflow-y: auto;
    background: rgba(0, 0, 0, 0.2);
  }

  .file-content pre {
    margin: 0;
    padding: 12px;
  }

  .file-content code {
    font-size: 12px;
    font-family: var(--mono, monospace);
    color: var(--text);
    white-space: pre;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 24px;
    border-top: 1px solid var(--border);
  }

  .btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 18px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    transition: background 0.15s;
  }

  .btn.deny {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
    border: 1px solid rgba(248, 113, 113, 0.3);
  }

  .btn.deny:hover {
    background: rgba(248, 113, 113, 0.2);
  }

  .btn.approve {
    background: rgba(74, 222, 128, 0.1);
    color: #4ade80;
    border: 1px solid rgba(74, 222, 128, 0.3);
  }

  .btn.approve:hover {
    background: rgba(74, 222, 128, 0.2);
  }
</style>
