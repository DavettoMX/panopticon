<script lang="ts">
  import { Send, Plus, Settings, Sparkles } from '@lucide/svelte';

  const suggestions = [
    { title: 'Help me study', subtitle: 'vocabulary for a college entrance exam' },
    { title: 'Give me ideas', subtitle: 'for what to do with my kids\u2019 art' },
    { title: 'Overcome procrastination', subtitle: 'give me tips' },
  ];
</script>

<main class="chat-area">
  <header class="chat-header">
    <div></div>
    <div class="header-actions">
      <button class="header-btn">
        <Settings size={18} />
      </button>
    </div>
  </header>

  <div class="chat-content">
    <div class="welcome">
      <div class="model-icon">P</div>
      <h1 class="model-name">llama3.2:3b</h1>
    </div>

    <div class="input-wrapper">
      <div class="message-input">
        <button class="input-action">
          <Plus size={18} />
        </button>
        <textarea
          rows={1}
          placeholder="How can I help you today?"
        ></textarea>
        <button class="send-btn">
          <Send size={18} />
        </button>
      </div>
    </div>

    <div class="suggestions">
      <div class="suggestions-header">
        <Sparkles size={14} />
        <span>Suggested</span>
      </div>
      {#each suggestions as suggestion}
        <button class="suggestion-item">
          <span class="suggestion-title">{suggestion.title}</span>
          <span class="suggestion-subtitle">{suggestion.subtitle}</span>
        </button>
      {/each}
    </div>
  </div>
</main>

<style>
  .chat-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    min-height: 52px;
  }

  .header-actions {
    display: flex;
    gap: 4px;
  }

  .header-btn {
    padding: 6px;
    border-radius: 6px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }

  .header-btn:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .chat-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 0 20px;
    gap: 24px;
    max-width: 640px;
    margin: 0 auto;
    width: 100%;
  }

  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .model-icon {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--primary);
    color: var(--text-bright);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
    font-weight: 700;
  }

  .model-name {
    font-size: 22px;
    font-weight: 500;
    color: var(--text-bright);
  }

  .input-wrapper {
    width: 100%;
  }

  .message-input {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    padding: 10px 14px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 12px;
    transition: border-color 0.15s;
  }

  .message-input:focus-within {
    border-color: var(--primary);
  }

  .message-input textarea {
    flex: 1;
    resize: none;
    font-size: 14px;
    line-height: 1.5;
    padding: 4px 0;
    min-height: 24px;
    max-height: 200px;
    background: transparent;
  }

  .message-input textarea::placeholder {
    color: var(--text-muted);
  }

  .input-action {
    padding: 4px;
    border-radius: 6px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }

  .input-action:hover {
    color: var(--text-bright);
  }

  .send-btn {
    padding: 6px;
    border-radius: 8px;
    background: var(--primary);
    color: var(--text-bright);
    display: flex;
    align-items: center;
    transition: background 0.15s;
  }

  .send-btn:hover {
    background: var(--primary-light);
  }

  .suggestions {
    width: 100%;
  }

  .suggestions-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 8px;
  }

  .suggestion-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    padding: 12px 16px;
    border-radius: 10px;
    margin-bottom: 4px;
    text-align: left;
    transition: background 0.1s;
  }

  .suggestion-item:hover {
    background: var(--bg-hover);
  }

  .suggestion-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-bright);
  }

  .suggestion-subtitle {
    font-size: 13px;
    color: var(--text-muted);
  }
</style>
