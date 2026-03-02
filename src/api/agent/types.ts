export type {
  CancelCommand,
  CommandRenderResult,
  CommandSummary,
  ExecuteTaskParams,
  FileContextStatus,
  SkillSource,
  SkillSummary,
  SkillValidationResult,
  TaskControlCommand,
  TaskListFilter,
  TaskProgressPayload,
  TaskProgressStream,
  TaskStatus,
  TaskSummary,
} from '@/types/domain/agent'

export const isTaskProgressEvent = (event: unknown): event is import('@/types/domain/agent').TaskProgressPayload => {
  if (!event || typeof event !== 'object') return false
  return typeof (event as { type?: unknown }).type === 'string'
}

export const isTerminalEvent = (event: import('@/types/domain/agent').TaskProgressPayload): boolean => {
  return event.type === 'task_completed' || event.type === 'task_cancelled' || event.type === 'task_error'
}

export const getEventTaskId = (event: import('@/types/domain/agent').TaskProgressPayload): string => {
  return 'taskId' in event && typeof event.taskId === 'string' ? event.taskId : ''
}

export const isErrorEvent = (event: import('@/types/domain/agent').TaskProgressPayload): boolean => {
  return event.type === 'task_error'
}
