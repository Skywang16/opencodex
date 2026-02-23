/**
 * Agent API type definitions
 *
 * Defines all interface types for Agent system, consistent with backend TaskExecutor
 */

import type { TaskEvent } from '@/types'

/**
 * Task execution parameters
 */
export interface ExecuteTaskParams {
  /** Workspace path */
  workspacePath: string
  /** Session ID */
  sessionId: number
  /** User prompt */
  userPrompt: string
  /** Model ID - required! */
  modelId: string
  /** Single request override agent type (will not persist to session) */
  agentType?: string
  /** Command ID for slash commands (e.g., "code-review", "skill-creator") */
  commandId?: string
  /** Image attachments (optional) */
  images?: Array<{ type: 'image'; dataUrl: string; mimeType: string }>
}

/**
 * Task summary information
 */
export interface TaskSummary {
  /** Task ID */
  taskId: string
  /** Session ID */
  sessionId: number
  /** Task status */
  status: TaskStatus
  /** Current iteration count */
  currentIteration: number
  /** Error count */
  errorCount: number
  /** Creation time */
  createdAt: string
  /** Update time */
  updatedAt: string
  /** User prompt */
  userPrompt?: string
  /** Completion time */
  completedAt?: string
}

/**
 * Task status
 */
export type TaskStatus =
  | 'created' // Created
  | 'running' // Running
  | 'paused' // Paused
  | 'completed' // Completed
  | 'error' // Error
  | 'cancelled' // Cancelled

/**
 * Task progress event payload (consistent with Rust TaskProgressPayload)
 */
export type TaskProgressPayload = TaskEvent

// ===== Streaming interface types =====

/**
 * Task progress stream interface
 *
 * Provides chainable event listening API
 */
export interface TaskProgressStream {
  /**
   * Listen to progress events
   * @param callback Progress callback function
   * @returns Stream object (supports chaining)
   */
  onProgress(callback: (event: TaskProgressPayload) => void): TaskProgressStream

  /**
   * Listen to error events
   * @param callback Error callback function
   * @returns Stream object (supports chaining)
   */
  onError(callback: (error: Error) => void): TaskProgressStream

  /**
   * Listen to stream close event
   * @param callback Close callback function
   * @returns Stream object (supports chaining)
   */
  onClose(callback: () => void): TaskProgressStream

  /**
   * Manually close stream
   */
  close(): void

  /**
   * Whether stream is closed
   */
  readonly isClosed: boolean
}

// ===== Control command types =====

/**
 * Task control command
 */
export type TaskControlCommand = CancelCommand

/**
 * Cancel command
 */
export interface CancelCommand {
  type: 'cancel'
  reason?: string
}

// ===== Query filter types =====

/**
 * Task list filter conditions
 */
export interface TaskListFilter {
  /** Session ID filter */
  sessionId?: number
  /** Status filter */
  status?: TaskStatus | string
  /** Pagination offset */
  offset?: number
  /** Pagination limit */
  limit?: number
}

// ===== Command system types =====

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

// ===== Skill system types =====

export type SkillSource = 'global' | 'workspace'

export interface SkillSummary {
  name: string
  description: string
  license?: string
  metadata: Record<string, string>
  /** Skill source: 'global' | 'workspace' */
  source: SkillSource
  /** Skill directory path */
  skillDir: string
}

export interface SkillValidationResult {
  valid: boolean
  errors: string[]
  warnings: string[]
}

// ===== Utility types =====

/**
 * Event type guard function
 */
export const isTaskProgressEvent = (event: unknown): event is TaskProgressPayload => {
  if (!event || typeof event !== 'object') {
    return false
  }

  const candidate = event as { type?: unknown }
  return typeof candidate.type === 'string'
}

/**
 * Determine if it is a terminal event
 */
export const isTerminalEvent = (event: TaskProgressPayload): boolean => {
  return event.type === 'task_completed' || event.type === 'task_cancelled' || event.type === 'task_error'
}

/**
 * Get task ID of event
 */
export const getEventTaskId = (event: TaskProgressPayload): string => {
  return 'taskId' in event && typeof event.taskId === 'string' ? event.taskId : ''
}

/**
 * Determine if it is an error event
 */
export const isErrorEvent = (event: TaskProgressPayload): boolean => {
  return event.type === 'task_error'
}

/**
 * File context status
 */
export interface FileContextStatus {
  workspacePath: string
  fileCount: number
  files: string[]
}
