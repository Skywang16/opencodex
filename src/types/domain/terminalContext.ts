export interface TerminalContext {
  paneId: number
  currentWorkingDirectory: string | null
  shellType: string | null
  shellIntegrationEnabled: boolean
  currentCommand: CommandInfo | null
  commandHistory: CommandInfo[]
  windowTitle: string | null
  lastActivity: string
  isActive: boolean
}

export interface CommandInfo {
  command: string
  args: string[]
  startTime: string
  endTime: string | null
  exitCode: number | null
  workingDirectory: string | null
}

export interface TerminalContextEvent {
  type: 'activePaneChanged' | 'paneContextUpdated' | 'paneCwdChanged' | 'paneShellIntegrationChanged'
  data: unknown
}

export interface ActivePaneChangedEvent {
  type: 'activePaneChanged'
  data: {
    oldPaneId: number | null
    newPaneId: number | null
  }
}

export interface PaneContextUpdatedEvent {
  type: 'paneContextUpdated'
  data: {
    paneId: number
    context: TerminalContext
  }
}

export interface PaneCwdChangedEvent {
  type: 'paneCwdChanged'
  data: {
    paneId: number
    oldCwd: string | null
    newCwd: string
  }
}

export interface PaneShellIntegrationChangedEvent {
  type: 'paneShellIntegrationChanged'
  data: {
    paneId: number
    enabled: boolean
  }
}

export interface ContextQueryOptions {
  useCache?: boolean
  timeout?: number
  allowFallback?: boolean
  includeHistory?: boolean
  maxHistoryCount?: number
}

export interface CacheStats {
  totalEntries: number
  hitCount: number
  missCount: number
  evictionCount: number
  hitRate: number
}

export interface TerminalContextResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
  code?: string
}
