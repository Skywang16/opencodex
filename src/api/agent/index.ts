/**
 * Agent API - Frontend interface wrapper for backend Agent system
 *
 * Provides task execution, state management, real-time progress monitoring, etc.
 */

import { agentChannelApi } from '@/api/channel/agent'
import { invoke } from '@/utils/request'
import type {
  CommandRenderResult,
  CommandSummary,
  ExecuteTaskParams,
  SkillSummary,
  SkillValidationResult,
  TaskListFilter,
  TaskProgressPayload,
  TaskProgressStream,
  TaskSummary,
} from './types'

/**
 * Agent API main class
 *
 * Wraps all functionality of backend TaskExecutor, providing type-safe interface
 */
export class AgentApi {
  /**
   * Execute Agent task
   * @param userPrompt User input
   * @param sessionId Session ID
   * @param modelId Model ID
   * @param images Image attachments (optional)
   * @returns Returns task progress stream
   */
  executeTask = async (params: ExecuteTaskParams): Promise<TaskProgressStream> => {
    const stream = agentChannelApi.createTaskStream(params)
    return this.createProgressStreamFromReadableStream(stream)
  }

  /**
   * Cancel task
   * @param taskId Task ID
   * @param reason Cancellation reason
   */
  cancelTask = async (taskId: string, reason?: string): Promise<void> => {
    await invoke('agent_cancel_task', { taskId, reason })
  }

  confirmTool = async (requestId: string, decision: 'allow_once' | 'allow_always' | 'deny'): Promise<void> => {
    await invoke('agent_tool_confirm', {
      params: { requestId, decision },
    })
  }

  /**
   * List tasks
   * @param filters Filter conditions
   * @returns Task summary list
   */
  listTasks = async (filters?: TaskListFilter): Promise<TaskSummary[]> => {
    return await invoke<TaskSummary[]>('agent_list_tasks', {
      sessionId: filters?.sessionId,
      statusFilter: filters?.status,
    })
  }

  listCommands = async (workspacePath: string): Promise<CommandSummary[]> => {
    return await invoke<CommandSummary[]>('agent_list_commands', {
      params: { workspacePath },
    })
  }

  renderCommand = async (workspacePath: string, name: string, input: string): Promise<CommandRenderResult> => {
    return await invoke<CommandRenderResult>('agent_render_command', {
      params: { workspacePath, name, input },
    })
  }

  listSkills = async (workspacePath: string): Promise<SkillSummary[]> => {
    return await invoke<SkillSummary[]>('agent_list_skills', {
      params: { workspacePath },
    })
  }

  validateSkill = async (skillPath: string): Promise<SkillValidationResult> => {
    return await invoke<SkillValidationResult>('agent_validate_skill', {
      skillPath,
    })
  }

  /**
   * Get task details
   * @param taskId Task ID
   * @returns Task detailed information
   */
  getTask = async (taskId: string): Promise<TaskSummary> => {
    const tasks = await this.listTasks()
    const task = tasks.find(t => t.taskId === taskId)

    if (!task) {
      throw new Error(`Task ${taskId} not found`)
    }

    return task
  }

  sendCommand = async (taskId: string, command: { type: 'cancel'; reason?: string }): Promise<void> => {
    await this.cancelTask(taskId, command.reason)
  }

  /**
   * Create task progress stream from ReadableStream
   * @private
   * @param stream ReadableStream
   * @returns TaskProgressStream
   */
  private createProgressStreamFromReadableStream(stream: ReadableStream<TaskProgressPayload>): TaskProgressStream {
    let isClosed = false
    const callbacks: Array<(event: TaskProgressPayload) => void> = []
    const errorCallbacks: Array<(error: Error) => void> = []
    const closeCallbacks: Array<() => void> = []
    let reader: ReadableStreamDefaultReader<TaskProgressPayload> | null = null
    // Used to temporarily store events when there are no subscribers yet, to avoid losing early events like TaskCreated
    const pendingEvents: TaskProgressPayload[] = []

    const startReading = async () => {
      try {
        reader = stream.getReader()

        while (!isClosed) {
          const { done, value } = await reader.read()

          if (done || isClosed) {
            closeStream()
            break
          }

          // Print Channel output content (using warn to comply with no-console rule)
          console.warn('[Channel output]', {
            type: value.type,
            data: value,
            timestamp: new Date().toISOString(),
          })

          if (callbacks.length === 0) {
            // No subscribers yet, temporarily store event
            pendingEvents.push(value)
          } else {
            // Notify all listeners
            callbacks.forEach(callback => {
              try {
                callback(value)
              } catch (error) {
                console.error('[AgentApi] Progress callback error:', error)
              }
            })
          }
        }
      } catch (error) {
        if (!isClosed) {
          errorCallbacks.forEach(callback => {
            try {
              callback(error as Error)
            } catch (err) {
              console.error('[AgentApi] Error callback error:', err)
            }
          })
          closeStream()
        }
      }
    }

    const closeStream = () => {
      if (isClosed) return
      isClosed = true

      if (reader) {
        reader.cancel().catch(console.error)
        reader = null
      }

      closeCallbacks.forEach(callback => {
        try {
          callback()
        } catch (error) {
          console.error('[AgentApi] Close callback error:', error)
        }
      })

      // Clear callback arrays
      callbacks.length = 0
      errorCallbacks.length = 0
      closeCallbacks.length = 0
    }

    startReading()

    // Create stream object
    const taskProgressStream: TaskProgressStream = {
      onProgress: callback => {
        if (!isClosed) {
          callbacks.push(callback)
          // When first subscriber appears, immediately replay pendingEvents to current subscriber
          if (pendingEvents.length > 0) {
            try {
              for (const ev of pendingEvents.splice(0, pendingEvents.length)) {
                callback(ev)
              }
            } catch (error) {
              console.error('[AgentApi] Replay stored event error:', error)
            }
          }
        }
        return taskProgressStream
      },

      onError: callback => {
        if (!isClosed) {
          errorCallbacks.push(callback)
        }
        return taskProgressStream
      },

      onClose: callback => {
        if (!isClosed) {
          closeCallbacks.push(callback)
        } else {
          callback()
        }
        return taskProgressStream
      },

      close: () => {
        closeStream()
      },

      get isClosed() {
        return isClosed
      },
    }

    return taskProgressStream
  }
}

/**
 * Agent API singleton instance
 */
export const agentApi = new AgentApi()

/**
 * Export types
 */
export * from './types'

/**
 * Frontend extension types
 */
export interface AgentTaskState extends TaskSummary {
  /** Whether listening to progress */
  isListening?: boolean
  /** Last update time */
  lastUpdated?: Date
  /** Progress stream reference */
  progressStream?: TaskProgressStream
  /** Recent progress events */
  recentEvents?: TaskProgressPayload[]
  /** Error information */
  error?: string
}

/**
 * Default export
 */
export default agentApi
