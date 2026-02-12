/**
 * Shell API types.
 *
 * ShellInfo and CreateTerminalWithShellOptions live in @/types/domain/terminal
 * as the single source of truth. Shell-specific types are defined here.
 */
export type { CreateTerminalWithShellOptions, ShellInfo } from '@/types/domain/terminal'

// ===== Shell feature types =====

export interface ShellFeatures {
  supportsColors: boolean
  supportsUnicode: boolean
  supportsTabCompletion: boolean
  supportsHistory: boolean
  supportsAliases: boolean
  supportsScripting: boolean
}

// ===== Background command execution =====

export interface BackgroundCommandResult {
  program: string
  args: string[]
  exitCode: number
  stdout: string
  stderr: string
  executionTimeMs: number
  success: boolean
}
