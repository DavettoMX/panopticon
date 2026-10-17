<script lang="ts">
  import { Rocket, ChevronDown, FolderOpen, Check, X } from '@lucide/svelte';
  import {
    orchStore,
    AGENT_ROLES,
    ROLE_COLORS,
    setAgentModel,
    initDefaultTeam,
    startOrchestration,
    validateWorkingDirectory,
    type AgentRole,
  } from '../orchestrator-store.svelte';
  import { store, formatModelDisplayName, formatQuantShort, getModelSizeClass } from '../stores.svelte';
  import VramPreview from './VramPreview.svelte';
  import { onMount } from 'svelte';

  let openDropdown = $state<string | null>(null);
  let workingDirInput = $state('');
  let validationTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    initDefaultTeam();
  });

  function handleWorkingDirInput(value: string) {
    workingDirInput = value;
    if (validationTimer) clearTimeout(validationTimer);
    if (!value.trim()) {
      orchStore.teamConfig.workingDirectory = '';
      orchStore.teamConfig.workingDirectoryValid = null;
      orchStore.teamConfig.workingDirectoryError = null;
      return;
    }
    validationTimer = setTimeout(() => {
      validateWorkingDirectory(value.trim());
    }, 500);
  }

  function toggleDropdown(role: string) {
    openDropdown = openDropdown === role ? null : role;
  }

  function selectModelForRole(role: AgentRole, modelPath: string) {
    setAgentModel(role, modelPath);
    openDropdown = null;
  }

  const allConfigured = $derived(
    orchStore.teamConfig.agents.every(a => a.model_path) &&
    orchStore.teamConfig.taskDescription.trim().length > 0
  );

  const judgeValid = $derived(() => {
    const judge = orchStore.teamConfig.agents.find(a => a.role === 'judge');
    const architect = orchStore.teamConfig.agents.find(a => a.role === 'architect');
    if (!judge || !architect) return true;
    return !judge.model_path || !architect.model_path || judge.model_path !== architect.model_path;
  });

  const canLaunch = $derived(
    allConfigured && judgeValid()
  );

  function getAssignedModelName(role: AgentRole): string {
    const agent = orchStore.teamConfig.agents.find(a => a.role === role);
    if (!agent?.model_path) return 'Select model...';
    const model = store.models.find(m => m.path === agent.model_path);
    return model ? formatModelDisplayName(model.filename) : agent.model_path;
  }
</script>

<div class="team-setup" data-cy="team-setup">
  <div class="setup-header">
    <h2>Orchestrator Mode</h2>
    <p class="setup-subtitle">Configure your agent team for collaborative software development</p>
  </div>

  <!-- Task Description -->
  <div class="task-section" data-cy="task-section">
    <label class="section-label" for="task-input">Task Description</label>
    <textarea
      id="task-input"
      class="task-input"
      placeholder="Describe the software development task for the team..."
      bind:value={orchStore.teamConfig.taskDescription}
      rows="3"
      data-cy="task-input"
    ></textarea>
  </div>

  <!-- Working Directory -->
  <div class="working-dir-section" data-cy="working-dir-section">
    <label class="section-label" for="working-dir-input">
      <FolderOpen size={12} />
      Working Directory
      <span class="optional-badge">optional</span>
    </label>
    <p class="section-hint">Set a directory where agents can create and edit files. You'll be asked for permission before any file is written.</p>
    <div class="working-dir-input-wrapper">
      <input
        id="working-dir-input"
        class="working-dir-input"
        class:valid={orchStore.teamConfig.workingDirectoryValid === true}
        class:invalid={orchStore.teamConfig.workingDirectoryValid === false}
        type="text"
        placeholder="/home/user/projects/my-project"
        value={workingDirInput}
        oninput={(e) => handleWorkingDirInput((e.target as HTMLInputElement).value)}
        data-cy="working-dir-input"
      />
      {#if orchStore.teamConfig.workingDirectoryValid === true}
        <span class="dir-status valid"><Check size={14} /></span>
      {:else if orchStore.teamConfig.workingDirectoryValid === false}
        <span class="dir-status invalid"><X size={14} /></span>
      {/if}
    </div>
    {#if orchStore.teamConfig.workingDirectoryError}
      <span class="dir-error">{orchStore.teamConfig.workingDirectoryError}</span>
    {/if}
  </div>

  <!-- Agent Cards Grid -->
  <div class="agents-grid">
    {#each AGENT_ROLES as roleInfo}
      {@const agent = orchStore.teamConfig.agents.find(a => a.role === roleInfo.value)}
      {@const color = ROLE_COLORS[roleInfo.value]}
      {@const isJudgeInvalid = roleInfo.value === 'judge' && !judgeValid()}
      <div
        class="agent-card"
        class:invalid={isJudgeInvalid}
        style="border-top: 3px solid {color}"
        data-cy="agent-card-{roleInfo.value}"
      >
        <div class="agent-card-header">
          <span class="agent-role-dot" style="background: {color}"></span>
          <span class="agent-role-name">{roleInfo.label}</span>
        </div>
        <p class="agent-description">{roleInfo.description}</p>

        <!-- Model Selector -->
        <div class="model-selector-wrapper">
          <button
            class="model-selector-btn"
            onclick={() => toggleDropdown(roleInfo.value)}
            data-cy="agent-model-btn-{roleInfo.value}"
          >
            <span class="model-selector-text">{getAssignedModelName(roleInfo.value)}</span>
            <ChevronDown size={14} />
          </button>

          {#if openDropdown === roleInfo.value}
            <div class="model-dropdown" data-cy="agent-model-dropdown-{roleInfo.value}">
              {#each store.models as model}
                {@const sizeClass = getModelSizeClass(model)}
                {@const quant = formatQuantShort(model.filename)}
                <button
                  class="model-option"
                  class:active={agent?.model_path === model.path}
                  onclick={() => selectModelForRole(roleInfo.value, model.path)}
                  data-cy="agent-model-option"
                >
                  <span class="model-option-name">{formatModelDisplayName(model.filename)}</span>
                  <span class="model-option-meta">
                    <span class="model-size-badge" class:slm={sizeClass === 'SLM'} class:llm={sizeClass === 'LLM'}>{sizeClass}</span>
                    {#if quant}
                      <span class="model-quant">{quant}</span>
                    {/if}
                    <span class="model-gb">{(model.size_bytes / 1073741824).toFixed(1)}G</span>
                  </span>
                </button>
              {/each}
              {#if store.models.length === 0}
                <div class="no-models">No models found. Check models directory in settings.</div>
              {/if}
            </div>
          {/if}
        </div>

        {#if isJudgeInvalid}
          <span class="validation-error">Judge must use a different model than the Architect</span>
        {/if}
      </div>
    {/each}
  </div>

  <!-- VRAM Preview -->
  <VramPreview />

  <!-- Launch Button -->
  <div class="launch-section">
    <button
      class="launch-btn"
      disabled={!canLaunch || orchStore.status === 'settingup'}
      onclick={startOrchestration}
      data-cy="launch-team-btn"
    >
      {#if orchStore.status === 'settingup'}
        <span class="spinner"></span>
        <span>Starting servers...</span>
      {:else}
        <Rocket size={18} />
        <span>Launch Team</span>
      {/if}
    </button>
    {#if orchStore.error}
      <div class="launch-error" data-cy="launch-error">{orchStore.error}</div>
    {/if}
  </div>
</div>

<style>
  .team-setup {
    max-width: 900px;
    margin: 0 auto;
    padding: 32px 24px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .setup-header h2 {
    font-size: 22px;
    font-weight: 600;
    color: var(--text-bright);
    margin-bottom: 4px;
  }

  .setup-subtitle {
    font-size: 13px;
    color: var(--text-muted);
  }

  .section-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: 8px;
  }

  .task-input {
    width: 100%;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-bright);
    padding: 12px;
    font-size: 14px;
    font-family: var(--sans);
    resize: vertical;
  }

  .task-input:focus {
    outline: none;
    border-color: var(--primary-light);
  }

  .task-input::placeholder {
    color: var(--text-muted);
  }

  .working-dir-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .section-hint {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
    line-height: 1.4;
  }

  .optional-badge {
    font-size: 10px;
    font-weight: 400;
    color: var(--text-muted);
    text-transform: lowercase;
    letter-spacing: 0;
    margin-left: 6px;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(255,255,255,0.05);
  }

  .working-dir-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .working-dir-input {
    width: 100%;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-bright);
    padding: 10px 36px 10px 12px;
    font-size: 13px;
    font-family: var(--mono, monospace);
  }

  .working-dir-input:focus {
    outline: none;
    border-color: var(--primary-light);
  }

  .working-dir-input.valid {
    border-color: #4ade80;
  }

  .working-dir-input.invalid {
    border-color: #f87171;
  }

  .working-dir-input::placeholder {
    color: var(--text-muted);
    font-family: var(--sans);
  }

  .dir-status {
    position: absolute;
    right: 10px;
    display: flex;
    align-items: center;
  }

  .dir-status.valid {
    color: #4ade80;
  }

  .dir-status.invalid {
    color: #f87171;
  }

  .dir-error {
    font-size: 11px;
    color: #f87171;
  }

  .agents-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 12px;
  }

  .agent-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition: border-color 0.15s;
  }

  .agent-card:hover {
    border-color: var(--primary-light);
  }

  .agent-card.invalid {
    border-color: #f87171;
  }

  .agent-card-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .agent-role-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .agent-role-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-bright);
  }

  .agent-description {
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .model-selector-wrapper {
    position: relative;
    margin-top: auto;
  }

  .model-selector-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 10px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-size: 12px;
    cursor: pointer;
    transition: border-color 0.15s;
  }

  .model-selector-btn:hover {
    border-color: var(--primary-light);
  }

  .model-selector-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    text-align: left;
  }

  .model-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    background: var(--bg-sidebar);
    border: 1px solid var(--border);
    border-radius: 8px;
    max-height: 200px;
    overflow-y: auto;
    z-index: 100;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
  }

  .model-option {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 8px 10px;
    font-size: 12px;
    color: var(--text);
    text-align: left;
    transition: background 0.1s;
  }

  .model-option:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .model-option.active {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .model-option-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-option-meta {
    display: flex;
    gap: 4px;
    align-items: center;
    flex-shrink: 0;
    margin-left: 8px;
  }

  .model-size-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 1px 4px;
    border-radius: 3px;
  }

  .model-size-badge.slm {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
  }

  .model-size-badge.llm {
    background: rgba(167, 139, 250, 0.15);
    color: #a78bfa;
  }

  .model-quant {
    font-size: 10px;
    color: var(--text-muted);
  }

  .model-gb {
    font-size: 10px;
    color: var(--text-muted);
  }

  .no-models {
    padding: 12px;
    font-size: 12px;
    color: var(--text-muted);
    text-align: center;
  }

  .validation-error {
    font-size: 11px;
    color: #f87171;
  }

  .launch-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .launch-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 32px;
    border-radius: 10px;
    background: var(--primary);
    color: var(--text-bright);
    font-size: 15px;
    font-weight: 600;
    transition: background 0.15s, opacity 0.15s;
  }

  .launch-btn:hover:not(:disabled) {
    background: var(--primary-light);
  }

  .launch-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255,255,255,0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .launch-error {
    font-size: 12px;
    color: #f87171;
    max-width: 500px;
    text-align: center;
  }
</style>
