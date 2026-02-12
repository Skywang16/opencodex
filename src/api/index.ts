/**
 * API unified export file
 *
 * Exports all submodule API instances and types uniformly,
 * providing convenient '@/api' import
 */

export { agentApi } from './agent'
export { agentTerminalApi } from './agent-terminal'
export { aiApi } from './ai'
export { appApi } from './app'
export { codeApi } from './code'
export { completionApi } from './completion'
export { configApi } from './config'
export { fileWatcherApi } from './file-watcher'
export { filesystemApi } from './filesystem'
export { gitApi } from './git'
export { llmApi } from './llm'
export { mcpApi } from './mcp'
export { nodeApi } from './node'
export { settingsApi } from './settings'
export { shellApi } from './shell'
export { shellIntegrationApi } from './shellIntegration'
export { shortcutsApi } from './shortcuts'
export { storageApi } from './storage'
export { terminalApi } from './terminal'
export { terminalContextApi } from './terminal-context'
export { vectorDbApi } from './vector-db'
export { windowApi } from './window'
export { workspaceApi } from './workspace'

export type * from './agent/types'
export type * from './ai/types'
export type { CodeDefinition } from './code'
export type * from './completion/types'
export type * from './git/types'
export type * from './mcp/types'
export type * from './shortcuts/types'
export type * from './storage/types'
export type * from './terminal-context/types'

// Export from config
export type { AppConfig, ConfigFileInfo } from './config/types'
// Export shell-specific types only (ShellInfo is re-exported from terminal)
export type { BackgroundCommandResult } from './shell/types'
// Export all types from terminal (includes ShellInfo)
export type * from './terminal/types'

export type { AgentApi } from './agent'
export type { AppApi } from './app'
export type { FilesystemApi } from './filesystem'
export type { LLMApi } from './llm'
export type { NodeApi, NodeVersionInfo } from './node'
export type { ShellIntegrationApi } from './shellIntegration'
export type { WindowApi } from './window'
