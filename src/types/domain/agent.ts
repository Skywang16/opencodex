import type { TaskEvent } from './aiMessage'

export interface ExecuteTaskParams {
  workspacePath: string
  sessionId: number
  userPrompt: string
  modelId: string
  agentType?: string
  commandId?: string
  images?: Array<{ type: 'image'; dataUrl: string; mimeType: string }>
}

export interface TaskSummary {
  taskId: string
  sessionId: number
  status: TaskStatus
  currentIteration: number
  errorCount: number
  createdAt: string
  updatedAt: string
  userPrompt?: string
  completedAt?: string
}

export type TaskStatus =
  | 'created'
  | 'running'
  | 'paused'
  | 'completed'
  | 'error'
  | 'cancelled'

export type TaskProgressPayload = TaskEvent

export interface TaskProgressStream {
  onProgress(callback: (event: TaskProgressPayload) => void): TaskProgressStream
  onError(callback: (error: Error) => void): TaskProgressStream
  onClose(callback: () => void): TaskProgressStream
  close(): void
  readonly isClosed: boolean
}

export type TaskControlCommand = CancelCommand

export interface CancelCommand {
  type: 'cancel'
  reason?: string
}

export interface TaskListFilter {
  sessionId?: number
  status?: TaskStatus | string
  offset?: number
  limit?: number
}

export interface CommandSummary {
  name: string
  description?: string
  agent?: string
  model?: string
  subtask: boolean
}

export interface CommandRenderResult {
  name: string
  agent?: string
  model?: string
  subtask: boolean
  prompt: string
}

export type SkillSource = 'global' | 'workspace'

export interface SkillSummary {
  name: string
  description: string
  license?: string
  metadata: Record<string, string>
  source: SkillSource
  skillDir: string
}

export interface SkillValidationResult {
  valid: boolean
  errors: string[]
  warnings: string[]
}

export interface FileContextStatus {
  workspacePath: string
  fileCount: number
  files: string[]
}
