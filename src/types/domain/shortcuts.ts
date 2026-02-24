/**
 * Shortcut domain type definitions
 */

// ===== Basic Shortcut Types =====

/**
 * Shortcut categories
 */
export enum ShortcutCategory {
  Global = 'global',
  Terminal = 'terminal',
  Tab = 'tab',
  AI = 'ai',
  Custom = 'custom',
}

export type ShortcutAction = string

export interface ShortcutBinding {
  key: string
  modifiers: string[]
  action: ShortcutAction
}

export type ShortcutsConfig = ShortcutBinding[]

export enum Platform {
  Windows = 'Windows',
  MacOS = 'MacOS',
  Linux = 'Linux',
}

// ===== Validation and Conflict Detection Types =====

export interface ShortcutValidationError {
  error_type: string
  message: string
  shortcut?: ShortcutBinding
}

export interface ShortcutValidationWarning {
  warning_type: string
  message: string
  shortcut?: ShortcutBinding
}

export interface ShortcutValidationResult {
  is_valid: boolean
  errors: ShortcutValidationError[]
  warnings: ShortcutValidationWarning[]
}

export interface ConflictingShortcut {
  category: string
  binding: ShortcutBinding
}

export interface ShortcutConflict {
  key_combination: string
  conflicting_shortcuts: ConflictingShortcut[]
}

export interface ConflictDetectionResult {
  has_conflicts: boolean
  conflicts: ShortcutConflict[]
}

// ===== Statistics and Search Types =====

export interface ShortcutStatistics {
  global_count: number
  terminal_count: number
  custom_count: number
  total_count: number
}

export interface ShortcutSearchOptions {
  query?: string
  categories?: ShortcutCategory[]
  key?: string
  modifiers?: string[]
  action?: string
}

// ===== Operation Option Types =====

export interface ShortcutOperationOptions {
  validate?: boolean
  checkConflicts?: boolean
  autoSave?: boolean
}

export interface ShortcutFormatOptions {
  platform?: Platform
  useSymbols?: boolean
  separator?: string
}

// ===== Frontend Extension Types =====

export type SupportedShortcutAction =
  | 'copy_to_clipboard'
  | 'paste_from_clipboard'
  | 'command_palette'
  | 'terminal_search'
  | 'open_settings'
  | 'new_terminal'
  | 'clear_terminal'
  | 'toggle_terminal_panel'
  | 'toggle_window_pin'

export interface ShortcutBackendResult {
  success: boolean
  message?: string
  data?: Record<string, unknown>
}

export interface ShortcutExecutionResult {
  success: boolean
  actionName: string
  keyCombo: string
  frontendResult?: boolean
  backendResult?: ShortcutBackendResult
  error?: string
}

export interface ShortcutListenerConfig {
  debugMode?: boolean
  autoStart?: boolean
  priority?: number
}

// ===== Event Types =====

export interface ShortcutEventData {
  conflictsWith?: ShortcutBinding[]
  oldBinding?: ShortcutBinding
  newBinding?: ShortcutBinding
  metadata?: Record<string, unknown>
}

export interface ShortcutEvent {
  type: 'shortcut_triggered' | 'shortcut_conflict' | 'shortcut_updated'
  shortcut?: ShortcutBinding
  data?: ShortcutEventData
  timestamp: number
}

export type ShortcutEventListener = (event: ShortcutEvent) => void
