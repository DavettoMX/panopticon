<script lang="ts">
  import { X, Cpu, MonitorCog, Zap, Gauge, Layers, ArrowRight } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import type { ModelInfo } from './stores.svelte';
  import { store, selectModel, setModelTags, ALL_MODEL_TAGS, getModelSizeClass, formatModelDisplayName } from './stores.svelte';

  interface GgufMetadata {
    architecture: string;
    quantization: string;
    parameter_count: number;
    layer_count: number;
    embedding_length: number;
    context_length: number;
    head_count: number;
    head_count_kv: number;
    vocab_size: number;
    file_type: number;
    tensor_count: number;
  }

  interface GpuInfo {
    name: string;
    vram_total_mb: number;
    vram_used_mb: number;
    vram_free_mb: number;
    compute_capability: string;
    pcie_bandwidth_gbps: number;
  }

  interface Props {
    model: ModelInfo;
    onclose: () => void;
  }

  let { model, onclose }: Props = $props();

  let metadata = $state<GgufMetadata | null>(null);
  let gpuInfo = $state<GpuInfo | null>(null);
  let systemRamMb = $state(0);
  let contextSize = $state(8192);
  let ngl = $state(0);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let starting = $state(false);
  let selectedTags = $state<string[]>([]);

  $effect(() => {
    if (model) {
      // Re-initialize tags when model changes
      selectedTags = [...(store.modelTags[model.path] ?? [])];
    }
  });

  const sizeClass = $derived(getModelSizeClass(model));

  function toggleTag(tag: string) {
    if (selectedTags.includes(tag)) {
      selectedTags = selectedTags.filter(t => t !== tag);
    } else {
      selectedTags = [...selectedTags, tag];
    }
    setModelTags(model.path, selectedTags);
  }

  const TAG_COLORS: Record<string, string> = {
    coding: '#22d3ee',
    chat: '#4ade80',
    vision: '#f472b6',
    reasoning: '#a78bfa',
    creative: '#fb923c',
    translation: '#38bdf8',
    summarization: '#facc15',
  };

  onMount(async () => {
    try {
      const [meta, gpu, ram, ctxSetting] = await Promise.all([
        invoke<GgufMetadata>('get_model_details', { modelPath: model.path }),
        invoke<GpuInfo>('get_gpu_info').catch(() => null),
        invoke<number>('get_system_ram_mb'),
        invoke<string | null>('get_setting', { key: 'context_size' }).catch(() => null),
      ]);
      metadata = meta;
      gpuInfo = gpu;
      systemRamMb = ram;
      if (ctxSetting) contextSize = parseInt(ctxSetting) || 8192;
      // Default: offload all layers to GPU
      ngl = meta.layer_count + 1; // +1 for output layer
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  // Quantization bits-per-weight lookup
  function bpwForQuant(quant: string): number {
    const map: Record<string, number> = {
      'F32': 32, 'F16': 16, 'BF16': 16,
      'Q8_0': 8.5, 'Q8_1': 9,
      'Q6_K': 6.56, 'Q5_K_M': 5.69, 'Q5_K_S': 5.54,
      'Q4_K_M': 4.85, 'Q4_K_S': 4.58, 'Q4_0': 4.5, 'Q4_1': 5,
      'Q3_K_L': 3.9, 'Q3_K_M': 3.6, 'Q3_K_S': 3.4,
      'Q2_K': 3.35,
      'IQ4_XS': 4.25, 'IQ4_NL': 4.5,
      'IQ3_S': 3.44, 'IQ3_XXS': 3.06,
      'IQ2_XS': 2.31, 'IQ2_XXS': 2.06, 'IQ2_S': 2.5,
      'IQ1_S': 1.56, 'IQ1_M': 1.75,
      'Q4_0_4_4': 4.5, 'Q4_0_4_8': 4.5, 'Q4_0_8_8': 4.5,
    };
    return map[quant] ?? 4.5;
  }

  // Per-layer weight size in MB
  const layerSizeMb = $derived.by(() => {
    if (!metadata) return 0;
    const totalLayers = metadata.layer_count + 1; // +1 output
    const totalModelMb = model.size_bytes / (1024 * 1024);
    return totalModelMb / totalLayers;
  });

  const totalLayers = $derived(metadata ? metadata.layer_count + 1 : 0);

  // GPU layers = ngl (clamped)
  const gpuLayers = $derived(Math.min(ngl, totalLayers));
  const cpuLayers = $derived(totalLayers - gpuLayers);

  // KV cache estimate (FP16): 2 (K+V) * layers * heads_kv * head_dim * context * 2 bytes
  const kvCacheMb = $derived.by(() => {
    if (!metadata) return 0;
    const headDim = metadata.embedding_length / metadata.head_count;
    const kvLayers = Math.min(gpuLayers, metadata.layer_count);
    // 2 for K+V, 2 bytes for FP16
    const bytes = 2 * kvLayers * metadata.head_count_kv * headDim * contextSize * 2;
    return bytes / (1024 * 1024);
  });

  // CUDA context overhead (~300MB) + compute buffer estimate
  const cudaOverheadMb = $derived(gpuLayers > 0 ? 300 : 0);

  // Memory estimates
  const gpuWeightsMb = $derived(gpuLayers * layerSizeMb);
  const gpuMemMb = $derived(gpuWeightsMb + kvCacheMb + cudaOverheadMb);
  const cpuMemMb = $derived(cpuLayers * layerSizeMb);

  const vramTotalMb = $derived(gpuInfo?.vram_total_mb ?? 0);
  const vramFreeMb = $derived(gpuInfo?.vram_free_mb ?? 0);
  const vramUsedByOtherMb = $derived(vramTotalMb - vramFreeMb);
  const vramAfterLoadMb = $derived(vramUsedByOtherMb + gpuMemMb);
  const vramOverflow = $derived(vramAfterLoadMb > vramTotalMb);

  const ramTotalMb = $derived(systemRamMb);

  // Bandwidth estimates
  const gpuBandwidthGbps = $derived.by(() => {
    if (!gpuInfo) return 0;
    // Approximate memory bandwidth based on GPU generation
    // RTX 5070 Ti ~ 896 GB/s, RTX 4090 ~ 1008 GB/s, etc.
    const cc = gpuInfo.compute_capability;
    const major = parseInt(cc.split('.')[0]);
    if (major >= 12) return 896; // Blackwell
    if (major >= 10) return 896; // Ada/Blackwell consumer
    if (major >= 9) return 1008; // Hopper
    if (major >= 8) return 640;  // Ampere
    return 448; // fallback
  });

  const pcieBandwidthGbps = $derived(gpuInfo?.pcie_bandwidth_gbps ?? 32);

  // Throughput estimate (t/s)
  // Simplified: inference is memory-bandwidth-bound
  // t/s ≈ effective_bandwidth / (model_size_per_token)
  // Per token: ~2 bytes per parameter for the active quantization
  const estimatedTps = $derived.by(() => {
    if (!metadata) return 0;
    const bpw = bpwForQuant(metadata.quantization);
    const bytesPerParam = bpw / 8;
    const modelSizeBytes = metadata.parameter_count * bytesPerParam;
    const modelSizeGb = modelSizeBytes / 1e9;

    if (gpuLayers === totalLayers) {
      // Full GPU: limited by GPU memory bandwidth
      return gpuBandwidthGbps / modelSizeGb;
    } else if (gpuLayers === 0) {
      // Full CPU: rough estimate ~2-8 t/s depending on model
      return Math.max(1, 50 / modelSizeGb);
    } else {
      // Mixed: limited by PCIe bandwidth for the transfer
      // The bottleneck is the slowest path
      const gpuFraction = gpuLayers / totalLayers;
      const effectiveBw = Math.min(
        gpuBandwidthGbps,
        pcieBandwidthGbps / (1 - gpuFraction)
      );
      return effectiveBw / modelSizeGb;
    }
  });

  function formatParams(n: number): string {
    if (n >= 1e12) return `${(n / 1e12).toFixed(1)}T`;
    if (n >= 1e9) return `${(n / 1e9).toFixed(1)}B`;
    if (n >= 1e6) return `${(n / 1e6).toFixed(0)}M`;
    return String(n);
  }

  function formatMb(mb: number): string {
    if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
    return `${Math.round(mb)} MB`;
  }

  function parseServerError(raw: string): string {
    if (raw.includes('cudaMalloc failed: out of memory') || raw.includes('failed to allocate')) {
      const layerInfo = gpuLayers < totalLayers
        ? ` Try reducing GPU layers (currently ${gpuLayers}/${totalLayers}).`
        : ' Try reducing GPU offload layers or lowering the context size.';
      return `Out of VRAM: the model requires more GPU memory than available.${layerInfo}`;
    }
    if (raw.includes('timed out')) {
      return 'The model server took too long to start. The model may be too large for your system.';
    }
    // Strip raw log noise but keep the essential message
    const match = raw.match(/llama-server crashed \(exit[^)]*\)/);
    if (match) {
      return `${match[0]}. Check the logs for details (~/.local/share/com.panopticon.app/llama-server.log).`;
    }
    return raw;
  }

  async function handleStart() {
    starting = true;
    const selectedModel = model;
    const selectedNgl = ngl === totalLayers ? null : ngl;
    store.serverStatus = 'loading';
    store.activeModel = selectedModel;
    store.serverError = null;
    onclose();
    try {
      await invoke('start_model', {
        modelPath: selectedModel.path,
        ngl: selectedNgl,
      });
      store.serverStatus = 'ready';
    } catch (e) {
      const errMsg = parseServerError(String(e));
      console.error('Failed to start model:', errMsg);
      store.serverStatus = 'stopped';
      store.serverError = errMsg;
    }
  }

  // Heatmap colors
  function layerColor(index: number): string {
    if (index < gpuLayers) {
      // GPU layer — gradient from purple to bright violet
      const t = gpuLayers > 1 ? index / (gpuLayers - 1) : 0;
      const r = Math.round(100 + t * 60);
      const g = Math.round(60 + t * 40);
      const b = Math.round(180 + t * 60);
      return `rgb(${r}, ${g}, ${b})`;
    } else {
      // CPU layer — blue-gray
      return '#3a4a5e';
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onclick={onclose}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="panel" onclick={(e) => e.stopPropagation()}>
    <header class="panel-header">
      <div class="panel-title">
        <h2>{formatModelDisplayName(model.filename)}</h2>
        <span class="size-class-badge {sizeClass.toLowerCase()}">{sizeClass}</span>
      </div>
      <button class="close-btn" onclick={onclose}>
        <X size={18} />
      </button>
    </header>

    {#if loading}
      <div class="panel-loading">
        <div class="spinner"></div>
        <span>Reading model metadata...</span>
      </div>
    {:else if error}
      <div class="panel-error">{error}</div>
    {:else if metadata}
      <div class="panel-body">
        <!-- Model info strip -->
        <div class="info-strip">
          <div class="info-chip">
            <span class="chip-label">Arch</span>
            <span class="chip-value">{metadata.architecture}</span>
          </div>
          <div class="info-chip">
            <span class="chip-label">Params</span>
            <span class="chip-value">{formatParams(metadata.parameter_count)}</span>
          </div>
          <div class="info-chip">
            <span class="chip-label">Quant</span>
            <span class="chip-value">{metadata.quantization}</span>
          </div>
          <div class="info-chip">
            <span class="chip-label">Layers</span>
            <span class="chip-value">{metadata.layer_count}</span>
          </div>
          <div class="info-chip">
            <span class="chip-label">Context</span>
            <span class="chip-value">{(metadata.context_length / 1024).toFixed(0)}K</span>
          </div>
          <div class="info-chip">
            <span class="chip-label">Size</span>
            <span class="chip-value">{(model.size_bytes / 1e9).toFixed(1)} GB</span>
          </div>
        </div>

        <!-- Task tags -->
        <div class="tags-section">
          <div class="tags-header">Task Categories</div>
          <div class="tags-grid">
            {#each ALL_MODEL_TAGS as { value, label }}
              {@const color = TAG_COLORS[value] ?? '#6b7280'}
              <button
                class="tag-toggle"
                class:active={selectedTags.includes(value)}
                style={selectedTags.includes(value) ? `color: ${color}; background: ${color}20; border-color: ${color}40` : ''}
                onclick={() => toggleTag(value)}
              >
                {label}
              </button>
            {/each}
          </div>
        </div>

        <!-- GPU offload slider -->
        <div class="offload-section">
          <div class="offload-header">
            <Layers size={16} />
            <span>GPU Offload</span>
            <span class="offload-value">{gpuLayers} / {totalLayers} layers</span>
          </div>

          <input
            type="range"
            min={0}
            max={totalLayers}
            bind:value={ngl}
            class="offload-slider"
          />

          <div class="slider-labels">
            <span>CPU only</span>
            <span>Full GPU</span>
          </div>
        </div>

        <!-- Layer heatmap -->
        <div class="heatmap-section">
          <div class="heatmap-header">
            <span>Layer Distribution</span>
          </div>
          <div class="heatmap">
            {#each Array(totalLayers) as _, i}
              <div
                class="heatmap-cell"
                style="background: {layerColor(i)}"
                title="Layer {i}: {i < gpuLayers ? 'GPU' : 'CPU'}"
              ></div>
            {/each}
          </div>
          <div class="heatmap-legend">
            <div class="legend-item">
              <div class="legend-swatch gpu"></div>
              <span>GPU ({gpuLayers})</span>
            </div>
            <div class="legend-item">
              <div class="legend-swatch cpu"></div>
              <span>CPU ({cpuLayers})</span>
            </div>
          </div>
        </div>

        <!-- Memory bars -->
        <div class="memory-section">
          {#if gpuInfo}
            <div class="memory-bar-group">
              <div class="memory-bar-header">
                <MonitorCog size={14} />
                <span>{gpuInfo.name}</span>
                <span class="memory-numbers" class:overflow={vramOverflow}>
                  {formatMb(gpuMemMb)} / {formatMb(vramTotalMb)}
                </span>
              </div>
              <div class="memory-bar">
                <div
                  class="memory-fill other"
                  style="width: {Math.min(100, (vramUsedByOtherMb / vramTotalMb) * 100)}%"
                  title="Used by other apps: {formatMb(vramUsedByOtherMb)}"
                ></div>
                <div
                  class="memory-fill gpu"
                  class:overflow={vramOverflow}
                  style="width: {Math.min(100 - (vramUsedByOtherMb / vramTotalMb) * 100, (gpuMemMb / vramTotalMb) * 100)}%"
                  title="Model: {formatMb(gpuMemMb)}"
                ></div>
              </div>
              {#if vramOverflow}
                <div class="overflow-warning">
                  Exceeds VRAM by {formatMb(vramAfterLoadMb - vramTotalMb)} (weights {formatMb(gpuWeightsMb)} + KV cache {formatMb(kvCacheMb)} + overhead {formatMb(cudaOverheadMb)})
                </div>
              {/if}
            </div>
          {/if}

          <div class="memory-bar-group">
            <div class="memory-bar-header">
              <Cpu size={14} />
              <span>System RAM</span>
              <span class="memory-numbers">
                {formatMb(cpuMemMb)} / {formatMb(ramTotalMb)}
              </span>
            </div>
            <div class="memory-bar">
              <div
                class="memory-fill ram"
                style="width: {Math.min(100, (cpuMemMb / ramTotalMb) * 100)}%"
              ></div>
            </div>
          </div>
        </div>

        <!-- Performance estimates -->
        <div class="perf-section">
          <div class="perf-card">
            <Gauge size={18} />
            <div class="perf-info">
              <span class="perf-value">{estimatedTps.toFixed(1)}</span>
              <span class="perf-label">est. tokens/s</span>
            </div>
          </div>
          <div class="perf-card">
            <Zap size={18} />
            <div class="perf-info">
              <span class="perf-value">
                {gpuLayers === totalLayers
                  ? `${gpuBandwidthGbps.toFixed(0)} GB/s`
                  : gpuLayers === 0
                    ? 'CPU mem'
                    : `${pcieBandwidthGbps.toFixed(0)} GB/s PCIe`}
              </span>
              <span class="perf-label">bandwidth</span>
            </div>
          </div>
        </div>

        <!-- Start button -->
        <button
          class="start-btn"
          class:disabled-btn={vramOverflow}
          onclick={handleStart}
          disabled={starting || vramOverflow}
          data-cy="load-model-btn"
        >
          {#if starting}
            Loading...
          {:else if vramOverflow}
            VRAM Insufficient
          {:else}
            Load Model
            <ArrowRight size={16} />
          {/if}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.15s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .panel {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 16px;
    width: 520px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.5);
    animation: slideUp 0.2s ease;
  }

  @keyframes slideUp {
    from { transform: translateY(20px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px 16px;
    border-bottom: 1px solid var(--border);
  }

  .panel-title {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .panel-header h2 {
    font-size: 17px;
    font-weight: 600;
    color: var(--text-bright);
    margin: 0;
  }

  .size-class-badge {
    font-size: 10px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 4px;
    letter-spacing: 0.5px;
  }

  .size-class-badge.slm {
    background: rgba(34, 211, 238, 0.15);
    color: #22d3ee;
  }

  .size-class-badge.llm {
    background: rgba(168, 85, 247, 0.15);
    color: #a855f7;
  }

  .close-btn {
    padding: 6px;
    border-radius: 8px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .panel-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 48px;
    color: var(--text-muted);
    font-size: 14px;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border);
    border-top-color: var(--primary-light);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .panel-error {
    padding: 24px;
    color: #f87171;
    font-size: 13px;
  }

  .panel-body {
    padding: 20px 24px 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  /* Info strip */
  .info-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .info-chip {
    display: flex;
    flex-direction: column;
    padding: 8px 12px;
    background: var(--bg-input);
    border-radius: 8px;
    min-width: 70px;
  }

  .chip-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: 2px;
  }

  .chip-value {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-bright);
  }

  /* Task tags */
  .tags-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .tags-header {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .tags-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tag-toggle {
    font-size: 12px;
    font-weight: 500;
    padding: 4px 12px;
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-muted);
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.15s;
  }

  .tag-toggle:hover {
    color: var(--text-bright);
    border-color: var(--border);
  }

  /* Offload slider */
  .offload-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .offload-header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-bright);
  }

  .offload-value {
    margin-left: auto;
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .offload-slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 6px;
    background: var(--bg-input);
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }

  .offload-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--primary-light);
    border: 2px solid var(--text-bright);
    cursor: pointer;
    transition: transform 0.1s;
  }

  .offload-slider::-webkit-slider-thumb:hover {
    transform: scale(1.15);
  }

  .offload-slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--primary-light);
    border: 2px solid var(--text-bright);
    cursor: pointer;
  }

  .slider-labels {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-muted);
  }

  /* Heatmap */
  .heatmap-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .heatmap-header {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .heatmap {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    padding: 10px;
    background: var(--bg-input);
    border-radius: 10px;
  }

  .heatmap-cell {
    width: 12px;
    height: 12px;
    border-radius: 2px;
    transition: background 0.15s ease;
  }

  .heatmap-legend {
    display: flex;
    gap: 16px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .legend-swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }

  .legend-swatch.gpu {
    background: rgb(140, 90, 220);
  }

  .legend-swatch.cpu {
    background: #3a4a5e;
  }

  /* Memory bars */
  .memory-section {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .memory-bar-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .memory-bar-header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text);
  }

  .memory-numbers {
    margin-left: auto;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }

  .memory-numbers.overflow {
    color: #f87171;
    font-weight: 600;
  }

  .memory-bar {
    height: 10px;
    background: var(--bg-input);
    border-radius: 5px;
    display: flex;
    overflow: hidden;
  }

  .memory-fill {
    height: 100%;
    transition: width 0.2s ease;
  }

  .memory-fill.other {
    background: #4a4a6a;
  }

  .memory-fill.gpu {
    background: linear-gradient(90deg, #7c3aed, #a855f7);
  }

  .memory-fill.gpu.overflow {
    background: linear-gradient(90deg, #dc2626, #f87171);
  }

  .memory-fill.ram {
    background: linear-gradient(90deg, #2563eb, #60a5fa);
  }

  .overflow-warning {
    font-size: 11px;
    color: #f87171;
    padding: 4px 8px;
    background: rgba(220, 38, 38, 0.1);
    border-radius: 4px;
  }

  /* Performance cards */
  .perf-section {
    display: flex;
    gap: 12px;
  }

  .perf-card {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    background: var(--bg-input);
    border-radius: 10px;
    color: var(--text-muted);
  }

  .perf-info {
    display: flex;
    flex-direction: column;
  }

  .perf-value {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-bright);
    font-variant-numeric: tabular-nums;
  }

  .perf-label {
    font-size: 11px;
    color: var(--text-muted);
  }

  /* Start button */
  .start-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 14px;
    border-radius: 10px;
    background: var(--primary);
    color: var(--text-bright);
    font-size: 15px;
    font-weight: 600;
    transition: background 0.15s;
  }

  .start-btn:hover:not(:disabled) {
    background: var(--primary-light);
  }

  .start-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .start-btn.disabled-btn {
    background: #7f1d1d;
    color: #f87171;
    opacity: 0.8;
  }
</style>
