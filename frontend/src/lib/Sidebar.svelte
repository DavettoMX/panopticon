<script lang="ts">
  import { SquarePen, ChevronDown, MessageSquare, PanelLeftClose, PanelLeft, Circle, Trash2, RotateCcw, X } from '@lucide/svelte';
  import {
    store,
    selectModel,
    newConversation,
    loadMessages,
    deleteConversation,
    restoreConversation,
    permanentDeleteConversation,
    type ModelInfo,
  } from './stores.svelte';

  let collapsed = $state(false);
  let showModelDropdown = $state(false);

  function handleModelSelect(model: ModelInfo) {
    selectModel(model);
    showModelDropdown = false;
  }

  function handleChatClick(id: string) {
    loadMessages(id);
  }

  async function handleNewChat() {
    await newConversation();
  }

  function daysUntilPurge(deletedAt: string | null): number {
    if (!deletedAt) return 30;
    const deleted = new Date(deletedAt + 'Z');
    const purgeDate = new Date(deleted.getTime() + 30 * 24 * 60 * 60 * 1000);
    const now = new Date();
    return Math.max(0, Math.ceil((purgeDate.getTime() - now.getTime()) / (24 * 60 * 60 * 1000)));
  }

  const statusColor = $derived(
    store.serverStatus === 'ready' ? '#4ade80' :
    store.serverStatus === 'loading' ? '#facc15' :
    '#6b7280'
  );
</script>

<aside class="sidebar" class:collapsed>
  <div class="sidebar-header">
    {#if !collapsed}
      <span class="logo">Panopticon</span>
    {/if}
    <button class="toggle-btn" onclick={() => collapsed = !collapsed}>
      {#if collapsed}
        <PanelLeft size={20} />
      {:else}
        <PanelLeftClose size={20} />
      {/if}
    </button>
  </div>

  {#if !collapsed}
    <button class="new-chat-btn" onclick={handleNewChat}>
      <SquarePen size={18} />
      <span>New Chat</span>
    </button>

    <div class="model-selector">
      <button class="model-btn" onclick={() => showModelDropdown = !showModelDropdown}>
        <span class="model-status">
          <Circle size={8} fill={statusColor} color={statusColor} />
        </span>
        <span class="model-name">
          {store.activeModel ? store.activeModel.filename.replace('.gguf', '') : 'Select model'}
        </span>
        <ChevronDown size={14} />
      </button>

      {#if showModelDropdown}
        <div class="model-dropdown">
          {#each store.models as model}
            <button
              class="model-option"
              class:active={store.activeModel?.path === model.path}
              onclick={() => handleModelSelect(model)}
            >
              <span>{model.filename.replace('.gguf', '')}</span>
              <span class="model-size">{(model.size_bytes / 1e9).toFixed(1)} GB</span>
            </button>
          {/each}
          {#if store.models.length === 0}
            <div class="model-option disabled">No models found</div>
          {/if}
        </div>
      {/if}
    </div>

    {#if store.serverError}
      <div class="server-error">
        {store.serverError}
      </div>
    {/if}

    <div class="chat-list">
      {#if !store.showTrash}
        {#if store.conversations.length > 0}
          <span class="chat-section-label">Conversations</span>
        {/if}
        {#each store.conversations as conv}
          <div class="chat-item-row" class:active={store.activeConversationId === conv.id}>
            <button
              class="chat-item"
              onclick={() => handleChatClick(conv.id)}
            >
              <MessageSquare size={16} />
              <span class="chat-title">{conv.title}</span>
            </button>
            <button
              class="chat-action delete-action"
              onclick={(e) => { e.stopPropagation(); deleteConversation(conv.id); }}
              title="Move to trash"
            >
              <Trash2 size={14} />
            </button>
          </div>
        {/each}
      {:else}
        <span class="chat-section-label">Trash</span>
        {#if store.trash.length === 0}
          <div class="trash-empty">Trash is empty</div>
        {/if}
        {#each store.trash as conv}
          <div class="chat-item-row trash-item">
            <div class="chat-item trash">
              <MessageSquare size={16} />
              <div class="trash-info">
                <span class="chat-title">{conv.title}</span>
                <span class="trash-days">{daysUntilPurge(conv.deleted_at)}d left</span>
              </div>
            </div>
            <div class="trash-actions">
              <button
                class="chat-action restore-action"
                onclick={() => restoreConversation(conv.id)}
                title="Restore"
              >
                <RotateCcw size={14} />
              </button>
              <button
                class="chat-action delete-action"
                onclick={() => permanentDeleteConversation(conv.id)}
                title="Delete permanently"
              >
                <X size={14} />
              </button>
            </div>
          </div>
        {/each}
      {/if}
    </div>

    <div class="sidebar-footer">
      <button class="trash-toggle" onclick={() => store.showTrash = !store.showTrash}>
        <Trash2 size={16} />
        <span>{store.showTrash ? 'Back to chats' : 'Trash'}</span>
        {#if !store.showTrash && store.trash.length > 0}
          <span class="trash-count">{store.trash.length}</span>
        {/if}
      </button>
      <div class="user-info">
        <div class="avatar">U</div>
        <span class="username">User</span>
      </div>
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    width: 260px;
    min-width: 260px;
    height: 100vh;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    transition: width 0.2s ease, min-width 0.2s ease;
  }

  .sidebar.collapsed {
    width: 50px;
    min-width: 50px;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    gap: 8px;
  }

  .collapsed .sidebar-header {
    justify-content: center;
    padding: 16px 8px;
  }

  .logo {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-bright);
    white-space: nowrap;
  }

  .toggle-btn {
    padding: 6px;
    border-radius: 6px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }

  .toggle-btn:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .new-chat-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 12px 8px;
    padding: 10px 12px;
    border-radius: 8px;
    color: var(--text-bright);
    font-size: 14px;
    background: var(--primary);
    transition: background 0.15s;
  }

  .new-chat-btn:hover {
    background: var(--primary-light);
  }

  .model-selector {
    padding: 0 12px;
    margin-bottom: 12px;
    position: relative;
  }

  .model-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text);
    background: var(--bg-input);
    border: 1px solid var(--border);
  }

  .model-btn:hover {
    border-color: var(--primary);
  }

  .model-status {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .model-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .model-dropdown {
    position: absolute;
    top: 100%;
    left: 12px;
    right: 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    margin-top: 4px;
    z-index: 100;
    overflow: hidden;
  }

  .model-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 12px;
    font-size: 13px;
    color: var(--text);
    text-align: left;
    transition: background 0.1s;
  }

  .model-option:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .model-option.active {
    color: var(--text-bright);
    background: var(--primary);
  }

  .model-option.disabled {
    color: var(--text-muted);
    cursor: default;
    padding: 12px;
    text-align: center;
    justify-content: center;
  }

  .model-size {
    font-size: 11px;
    color: var(--text-muted);
  }

  .chat-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px;
  }

  .chat-section-label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    padding: 12px 12px 6px;
  }

  .chat-item-row {
    display: flex;
    align-items: center;
    border-radius: 8px;
    transition: background 0.1s;
  }

  .chat-item-row:hover {
    background: var(--bg-hover);
  }

  .chat-item-row.active {
    background: var(--bg-hover);
  }

  .chat-item-row:hover .chat-action {
    opacity: 1;
  }

  .chat-item {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 0;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text);
    text-align: left;
  }

  .chat-item-row:hover .chat-item,
  .chat-item-row.active .chat-item {
    color: var(--text-bright);
  }

  .chat-item.trash {
    cursor: default;
  }

  .chat-action {
    padding: 4px;
    border-radius: 4px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    opacity: 0;
    transition: opacity 0.15s, color 0.15s;
    flex-shrink: 0;
    margin-right: 4px;
  }

  .chat-action:hover {
    color: var(--text-bright);
  }

  .delete-action:hover {
    color: #f87171;
  }

  .restore-action:hover {
    color: #4ade80;
  }

  .trash-item .chat-action {
    opacity: 1;
  }

  .trash-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
    margin-right: 4px;
  }

  .trash-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .trash-days {
    font-size: 10px;
    color: var(--text-muted);
  }

  .trash-empty {
    padding: 12px;
    font-size: 13px;
    color: var(--text-muted);
    text-align: center;
  }

  .chat-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sidebar-footer {
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .trash-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 8px;
    border-radius: 6px;
    font-size: 13px;
    color: var(--text-muted);
    transition: all 0.15s;
  }

  .trash-toggle:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .trash-count {
    margin-left: auto;
    background: rgba(255, 255, 255, 0.1);
    padding: 1px 6px;
    border-radius: 10px;
    font-size: 11px;
  }

  .user-info {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .avatar {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: var(--primary);
    color: var(--text-bright);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 600;
  }

  .username {
    font-size: 13px;
    color: var(--text-bright);
  }

  .server-error {
    padding: 6px 12px;
    margin: 0 12px 8px;
    font-size: 11px;
    color: #f87171;
    background: rgba(248, 113, 113, 0.1);
    border-radius: 6px;
    word-break: break-word;
  }
</style>
