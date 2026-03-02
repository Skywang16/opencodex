export interface ShellFeatures {
  supportsColors: boolean
  supportsUnicode: boolean
  supportsTabCompletion: boolean
  supportsHistory: boolean
  supportsAliases: boolean
  supportsScripting: boolean
}

export interface BackgroundCommandResult {
  program: string
  args: string[]
  exitCode: number
  stdout: string
  stderr: string
  executionTimeMs: number
  success: boolean
}
