export enum StorageLayer {
  Config = 'config',
  State = 'state',
  Data = 'data',
}

export enum ConfigSection {
  App = 'app',
  Appearance = 'appearance',
  Terminal = 'terminal',
  Shortcuts = 'shortcuts',
  Ai = 'ai',
}

export type RuntimeTerminalKind = 'workspace' | 'task'
export type TaskTerminalMode = 'blocking' | 'background'
export type TaskTerminalStatus = 'initializing' | 'running' | 'completed' | 'failed' | 'aborted'

export interface DataQuery {
  query: string
  params: Record<string, unknown>
  limit?: number
  offset?: number
  order_by?: string
  desc: boolean
}

export interface SaveOptions {
  table?: string
  overwrite: boolean
  backup: boolean
  validate: boolean
  metadata: Record<string, unknown>
}

export interface WindowGeometry {
  x: number
  y: number
  width: number
  height: number
  maximized: boolean
}

/**
 * Runtime terminal state (queried from backend)
 */
export interface RuntimeTerminalState {
  id: number
  cwd: string
  shell: string
  /** Pre-computed display title for the tab */
  displayTitle: string
  kind: RuntimeTerminalKind
  sessionId: number | null
  taskTerminalId: string | null
  sourceLabel: string | null
  taskMode: TaskTerminalMode | null
  taskStatus: TaskTerminalStatus | null
}

export interface StorageEvent {
  type: 'config_changed' | 'data_updated' | 'error'
  data: unknown
  timestamp: number
}

export interface StorageOperationResult {
  success: boolean
  error?: string
}
