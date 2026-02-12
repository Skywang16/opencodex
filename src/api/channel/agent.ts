import type { ExecuteTaskParams, TaskProgressPayload } from '@/api/agent/types'
import { channelApi } from './index'

/**
 * Agent-specific Channel API
 */
class AgentChannelApi {
  /**
   * Create Agent task execution stream
   */
  createTaskStream = (params: ExecuteTaskParams): ReadableStream<TaskProgressPayload> => {
    // The backend may emit task_* events for subtasks on the same event stream.
    // Only close this stream when the *root* task (the one created by agent_execute_task) ends.
    let rootTaskId: string | null = null
    return channelApi.createStream<TaskProgressPayload>(
      'agent_execute_task',
      { params },
      {
        cancelCommand: 'agent_cancel_task',
        shouldClose: (event: TaskProgressPayload) => {
          if (event.type === 'task_created') {
            rootTaskId = event.taskId
            return false
          }
          if (!rootTaskId) return false
          if (event.type === 'task_completed' || event.type === 'task_cancelled' || event.type === 'task_error') {
            return event.taskId === rootTaskId
          }
          return false
        },
      }
    )
  }

  /**
   * Resume task removed (no longer supported)
   */
}

export const agentChannelApi = new AgentChannelApi()
