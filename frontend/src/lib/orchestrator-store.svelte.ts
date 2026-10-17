import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// --- Types ---

export type AgentRole = 'orchestrator' | 'architect' | 'auditor' | 'constructor' | 'judge';
export type OrchestratorStatus = 'idle' | 'settingup' | 'running' | 'paused' | 'complete' | 'error';
export type AgentStatus = 'idle' | 'thinking' | 'done' | 'error';

export const AGENT_ROLES: { value: AgentRole; label: string; description: string }[] = [
  { value: 'orchestrator', label: 'Orchestrator', description: 'Coordinates the team, divides work, tracks plan' },
  { value: 'architect', label: 'Architect', description: 'Designs architecture, researches solutions' },
  { value: 'auditor', label: 'Auditor', description: 'Reviews codebase, finds files, answers structure questions' },
  { value: 'constructor', label: 'Constructor', description: 'Writes code following Architect directives' },
  { value: 'judge', label: 'Judge', description: 'Reviews plans for gaps and edge cases' },
];

export const ROLE_COLORS: Record<AgentRole, string> = {
  orchestrator: '#a78bfa', // purple
  architect: '#22d3ee',    // cyan
  auditor: '#4ade80',      // green
  constructor: '#fb923c',  // orange
  judge: '#f472b6',        // pink
};

export interface AgentConfig {
  role: AgentRole;
  model_path: string;
  ngl: number | null;
  context_size: number;
  system_prompt: string | null;
}

export interface TeamConfig {
  agents: AgentConfig[];
  task_description: string;
  working_directory: string | null;
}

export interface AgentVramEstimate {
  role: string;
  model_filename: string;
  gpu_mem_mb: number;
  cpu_mem_mb: number;
  needs_cpu_offload: boolean;
  ngl: number;
  total_layers: number;
}

export interface VramEstimate {
  agents: AgentVramEstimate[];
  total_gpu_mb: number;
  total_cpu_mb: number;
  gpu_available_mb: number;
  will_overflow: boolean;
  overflow_mb: number;
}

export interface OrchestratorSession {
  id: string;
  task_description: string;
  status: string;
  created_at: string;
  completed_at: string | null;
}

export interface OrchestratorMessageRow {
  id: string;
  session_id: string;
  from_role: string;
  to_role: string;
  content: string;
  message_type: string;
  step_number: number;
  created_at: string;
}

export interface AgentTokenPayload {
  role: string;
  content: string;
}

export interface AgentStartPayload {
  role: string;
  instruction: string;
  step_number: number;
}

export interface AgentCompletePayload {
  role: string;
  response: string;
  step_number: number;
}

export interface CheckpointPayload {
  summary: string;
  step_number: number;
}

export interface OrchestratorCompletePayload {
  final_summary: string;
}

export interface OrchestratorErrorPayload {
  error: string;
  role: string | null;
}

export interface AgentSwapPayload {
  role: string;
  model_filename: string;
}

export interface FileProposal {
  path: string;
  content: string;
  is_new: boolean;
}

export interface FileProposalPayload {
  files: FileProposal[];
  step_number: number;
}

export interface FilesWrittenPayload {
  success: boolean;
  summary: string;
}

export interface ToolCallPayload {
  role: string;
  tool_name: string;
  arguments: string;
  step_number: number;
}

// --- Reactive state ---

export const orchStore = $state({
  mode: 'chat' as 'chat' | 'orchestrator',

  // Team setup
  teamConfig: {
    agents: [] as AgentConfig[],
    taskDescription: '',
    workingDirectory: '' as string,
    workingDirectoryValid: null as boolean | null,
    workingDirectoryError: null as string | null,
  },

  // Runtime
  sessionId: null as string | null,
  status: 'idle' as OrchestratorStatus,
  executionLog: [] as OrchestratorMessageRow[],
  activeAgent: null as string | null,
  agentStreaming: {} as Record<string, string>,
  agentStatuses: {} as Record<string, AgentStatus>,
  currentStep: 0,
  checkpointSummary: null as string | null,

  // Swap state
  swappingAgent: null as string | null,

  // VRAM
  vramEstimate: null as VramEstimate | null,

  // File proposals (permission dialog)
  pendingFileProposal: null as FileProposalPayload | null,

  // UI state
  expandedAgents: new Set<string>() as Set<string>,
  sessions: [] as OrchestratorSession[],
  error: null as string | null,
});

// --- Functions ---

export function switchMode(mode: 'chat' | 'orchestrator') {
  orchStore.mode = mode;
}

export function initDefaultTeam() {
  if (orchStore.teamConfig.agents.length > 0) return;
  orchStore.teamConfig.agents = AGENT_ROLES.map(r => ({
    role: r.value,
    model_path: '',
    ngl: null,
    context_size: 8192,
    system_prompt: null,
  }));
}

export function setAgentModel(role: AgentRole, modelPath: string) {
  const agent = orchStore.teamConfig.agents.find(a => a.role === role);
  if (agent) {
    agent.model_path = modelPath;
  }
  estimateTeamVram();
}

export function setAgentNgl(role: AgentRole, ngl: number | null) {
  const agent = orchStore.teamConfig.agents.find(a => a.role === role);
  if (agent) {
    agent.ngl = ngl;
  }
  estimateTeamVram();
}

export async function validateWorkingDirectory(path: string) {
  if (!path.trim()) {
    orchStore.teamConfig.workingDirectoryValid = null;
    orchStore.teamConfig.workingDirectoryError = null;
    orchStore.teamConfig.workingDirectory = '';
    return;
  }

  try {
    const result = await invoke<{ valid: boolean; error?: string }>('validate_working_directory', { path });
    orchStore.teamConfig.workingDirectoryValid = result.valid;
    orchStore.teamConfig.workingDirectoryError = result.valid ? null : (result.error || 'Invalid directory');
    if (result.valid) {
      orchStore.teamConfig.workingDirectory = path;
    }
  } catch (e) {
    orchStore.teamConfig.workingDirectoryValid = false;
    orchStore.teamConfig.workingDirectoryError = String(e);
  }
}

export async function approveFileWrites() {
  try {
    await invoke('approve_file_writes');
    orchStore.pendingFileProposal = null;
  } catch (e) {
    console.error('Failed to approve file writes:', e);
  }
}

export async function denyFileWrites() {
  try {
    await invoke('deny_file_writes');
    orchStore.pendingFileProposal = null;
  } catch (e) {
    console.error('Failed to deny file writes:', e);
  }
}

export async function estimateTeamVram() {
  const validAgents = orchStore.teamConfig.agents.filter(a => a.model_path);
  if (validAgents.length === 0) {
    orchStore.vramEstimate = null;
    return;
  }

  try {
    orchStore.vramEstimate = await invoke<VramEstimate>('estimate_team_vram', {
      teamConfig: {
        agents: validAgents,
        task_description: orchStore.teamConfig.taskDescription || '',
      },
    });
  } catch (e) {
    console.error('Failed to estimate VRAM:', e);
    orchStore.vramEstimate = null;
  }
}

export async function startOrchestration() {
  const validAgents = orchStore.teamConfig.agents.filter(a => a.model_path);
  if (validAgents.length === 0 || !orchStore.teamConfig.taskDescription.trim()) return;

  orchStore.error = null;
  orchStore.executionLog = [];
  orchStore.agentStreaming = {};
  orchStore.agentStatuses = {};
  orchStore.currentStep = 0;
  orchStore.checkpointSummary = null;

  // Reset agent statuses
  for (const agent of validAgents) {
    orchStore.agentStatuses[agent.role] = 'idle';
  }

  const workingDir = orchStore.teamConfig.workingDirectory.trim() || null;

  try {
    // Create session
    const sessionId = await invoke<string>('create_orchestrator_session', {
      teamConfig: {
        agents: validAgents,
        task_description: orchStore.teamConfig.taskDescription,
        working_directory: workingDir,
      },
    });
    orchStore.sessionId = sessionId;
    orchStore.status = 'settingup';

    // Start orchestration
    await invoke('start_orchestrator', {
      sessionId,
      teamConfig: {
        agents: validAgents,
        task_description: orchStore.teamConfig.taskDescription,
        working_directory: workingDir,
      },
    });

    orchStore.status = 'running';
  } catch (e) {
    console.error('Failed to start orchestration:', e);
    orchStore.error = String(e);
    orchStore.status = 'error';
  }
}

export async function stopOrchestration() {
  try {
    await invoke('stop_orchestrator');
    orchStore.status = 'idle';
    orchStore.sessionId = null;
  } catch (e) {
    console.error('Failed to stop orchestration:', e);
  }
}

export async function pauseOrchestration() {
  try {
    await invoke('pause_orchestrator');
    orchStore.status = 'paused';
  } catch (e) {
    console.error('Failed to pause:', e);
  }
}

export async function resumeOrchestration() {
  try {
    await invoke('resume_orchestrator');
    orchStore.status = 'running';
    orchStore.checkpointSummary = null;
  } catch (e) {
    console.error('Failed to resume:', e);
  }
}

export async function injectUserMessage(content: string) {
  try {
    await invoke('inject_user_message', { content });
    orchStore.status = 'running';
    orchStore.checkpointSummary = null;
  } catch (e) {
    console.error('Failed to inject message:', e);
  }
}


export async function deleteSession(sessionId: string) {
  try {
    await invoke("delete_orchestrator_session", { sessionId });
    if (orchStore.sessionId === sessionId) {
      orchStore.sessionId = null;
      orchStore.executionLog = [];
      orchStore.status = "idle";
      orchStore.currentStep = 0;
    }
    await loadSessions();
  } catch (e) {
    console.error("Failed to delete session:", e);
  }
}

export async function loadSessions() {
  try {
    orchStore.sessions = await invoke<OrchestratorSession[]>('list_orchestrator_sessions');
  } catch (e) {
    console.error('Failed to load sessions:', e);
  }
}

export async function loadSessionLog(sessionId: string) {
  try {
    orchStore.executionLog = await invoke<OrchestratorMessageRow[]>('get_orchestrator_log', { sessionId });
  } catch (e) {
    console.error('Failed to load session log:', e);
  }
}

// --- Event listeners ---

let unlisteners: UnlistenFn[] = [];

export function setupOrchestratorEvents() {
  listen<AgentSwapPayload>('orchestrator-agent-swap', (event) => {
    orchStore.swappingAgent = event.payload.role.toLowerCase();
  }).then(fn => unlisteners.push(fn));

  listen<AgentStartPayload>('orchestrator-agent-start', (event) => {
    const { role, instruction, step_number } = event.payload;
    orchStore.swappingAgent = null;
    orchStore.activeAgent = role;
    orchStore.currentStep = step_number;
    // Force full object reassignment for reactivity
    orchStore.agentStatuses = { ...orchStore.agentStatuses, [role.toLowerCase()]: 'thinking' as AgentStatus };
    orchStore.agentStreaming = { ...orchStore.agentStreaming, [role.toLowerCase()]: '' };
    orchStore.expandedAgents = new Set([...orchStore.expandedAgents, role.toLowerCase()]);
    orchStore.executionLog = [...orchStore.executionLog, {
      id: crypto.randomUUID(),
      session_id: orchStore.sessionId || '',
      from_role: 'orchestrator',
      to_role: role.toLowerCase(),
      content: instruction,
      message_type: 'instruction',
      step_number,
      created_at: new Date().toISOString(),
    }];
  }).then(fn => unlisteners.push(fn));

  listen<AgentTokenPayload>('orchestrator-agent-token', (event) => {
    const { role, content } = event.payload;
    const key = role.toLowerCase();
    
    if (orchStore.swappingAgent === key) {
      orchStore.swappingAgent = null;
    }

    const prev = orchStore.agentStreaming[key] || '';
    orchStore.agentStreaming = { ...orchStore.agentStreaming, [key]: prev + content };
  }).then(fn => unlisteners.push(fn));

  listen<AgentCompletePayload>('orchestrator-agent-complete', (event) => {
    const { role, response, step_number } = event.payload;
    const key = role.toLowerCase();
    
    if (orchStore.swappingAgent === key) {
      orchStore.swappingAgent = null;
    }

    orchStore.agentStatuses = { ...orchStore.agentStatuses, [key]: 'done' as AgentStatus };
    orchStore.agentStreaming = { ...orchStore.agentStreaming, [key]: '' };

    if (orchStore.activeAgent?.toLowerCase() === key) {
      orchStore.activeAgent = null;
    }

    orchStore.executionLog = [...orchStore.executionLog, {
      id: crypto.randomUUID(),
      session_id: orchStore.sessionId || '',
      from_role: key,
      to_role: 'orchestrator',
      content: response,
      message_type: 'response',
      step_number,
      created_at: new Date().toISOString(),
    }];
  }).then(fn => unlisteners.push(fn));

  listen<CheckpointPayload>('orchestrator-checkpoint', (event) => {
    const { summary, step_number } = event.payload;
    orchStore.status = 'paused';
    orchStore.checkpointSummary = summary;
    orchStore.currentStep = step_number;

    orchStore.executionLog = [...orchStore.executionLog, {
      id: crypto.randomUUID(),
      session_id: orchStore.sessionId || '',
      from_role: 'orchestrator',
      to_role: 'system',
      content: summary,
      message_type: 'checkpoint',
      step_number,
      created_at: new Date().toISOString(),
    }];
  }).then(fn => unlisteners.push(fn));

  listen<OrchestratorCompletePayload>('orchestrator-complete', (event) => {
    orchStore.status = 'complete';

    orchStore.executionLog = [...orchStore.executionLog, {
      id: crypto.randomUUID(),
      session_id: orchStore.sessionId || '',
      from_role: 'orchestrator',
      to_role: 'system',
      content: event.payload.final_summary,
      message_type: 'complete',
      step_number: orchStore.currentStep,
      created_at: new Date().toISOString(),
    }];
  }).then(fn => unlisteners.push(fn));

  listen<ToolCallPayload>('orchestrator-tool-call', (event) => {
    const { role, tool_name, step_number } = event.payload;
    orchStore.executionLog = [...orchStore.executionLog, {
      id: crypto.randomUUID(),
      session_id: orchStore.sessionId || '',
      from_role: role.toLowerCase(),
      to_role: 'system',
      content: `Calling tool: **${tool_name}**`,
      message_type: 'tool_call',
      step_number,
      created_at: new Date().toISOString(),
    }];
  }).then(fn => unlisteners.push(fn));

  listen<FileProposalPayload>('orchestrator-file-proposal', (event) => {
    orchStore.pendingFileProposal = event.payload;
  }).then(fn => unlisteners.push(fn));

  listen<FilesWrittenPayload>('orchestrator-files-written', (event) => {
    const { success, summary } = event.payload;
    orchStore.executionLog = [...orchStore.executionLog, {
      id: crypto.randomUUID(),
      session_id: orchStore.sessionId || '',
      from_role: 'system',
      to_role: 'system',
      content: summary,
      message_type: success ? 'file_write' : 'error',
      step_number: orchStore.currentStep,
      created_at: new Date().toISOString(),
    }];
  }).then(fn => unlisteners.push(fn));

  listen<OrchestratorErrorPayload>('orchestrator-error', (event) => {
    orchStore.status = 'error';
    orchStore.error = event.payload.error;
    if (event.payload.role) {
      orchStore.agentStatuses = { ...orchStore.agentStatuses, [event.payload.role.toLowerCase()]: 'error' as AgentStatus };
    }
  }).then(fn => unlisteners.push(fn));
}

export function cleanupOrchestratorEvents() {
  for (const unlisten of unlisteners) {
    unlisten();
  }
  unlisteners = [];
}
