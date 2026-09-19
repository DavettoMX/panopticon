<script lang="ts">
  import { SquarePen, ChevronDown, MessageSquare, PanelLeftClose, PanelLeft } from '@lucide/svelte';

  let collapsed = $state(false);

  const models = ['llama3.2:3b', 'gemma2:9b', 'qwen2.5:7b'];
  let selectedModel = $state(models[0]);

  const chats = [
    { id: 1, title: 'Explaining vector search' },
    { id: 2, title: 'Diagnosing an afternoon slo...' },
    { id: 3, title: 'Why migrations must be bac...' },
    { id: 4, title: 'Debugging Rust lifetimes' },
  ];
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
    <button class="new-chat-btn">
      <SquarePen size={18} />
      <span>New Chat</span>
    </button>

    <div class="model-selector">
      <button class="model-btn">
        <span class="model-name">{selectedModel}</span>
        <ChevronDown size={14} />
      </button>
    </div>

    <div class="chat-list">
      <span class="chat-section-label">Today</span>
      {#each chats as chat}
        <button class="chat-item">
          <MessageSquare size={16} />
          <span class="chat-title">{chat.title}</span>
        </button>
      {/each}
    </div>

    <div class="sidebar-footer">
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
  }

  .model-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
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

  .model-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .chat-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text);
    text-align: left;
    transition: background 0.1s;
  }

  .chat-item:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
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
</style>
