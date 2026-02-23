/**
 * Workspace management API
 *
 * Unified workspace-related functionality including:
 * - Workspace CRUD operations
 * - Session management
 * - Recent workspace tracking
 * - Project rules management
 */

import type { Message } from '@/types'
import { invoke } from '@/utils/request'

export interface RecentWorkspace {
  id: number
  path: string
  last_accessed_at: number
}

export interface WorkspaceRecord {
  path: string
  displayName?: string | null
  activeSessionId?: number | null
  selectedRunActionId?: string | null
  createdAt: number
  updatedAt: number
  lastAccessedAt: number
}

export interface SessionRecord {
  id: number
  workspacePath: string
  title?: string | null
  messageCount: number
  createdAt: number
  updatedAt: number
}

export interface RunActionRecord {
  id: string
  workspacePath: string
  name: string
  command: string
  sortOrder: number
}

/**
 * Unified Workspace API
 */
export class WorkspaceApi {
  // ===== Workspace operations =====

  getOrCreate = async (path: string): Promise<WorkspaceRecord> => {
    return invoke<WorkspaceRecord>('workspace_get_or_create', { path })
  }

  deleteWorkspace = async (path: string): Promise<void> => {
    await invoke('workspace_remove_recent', { path })
  }

  // ===== Session operations =====

  listSessions = async (path: string): Promise<SessionRecord[]> => {
    return invoke<SessionRecord[]>('workspace_list_sessions', { path })
  }

  createSession = async (path: string, title?: string): Promise<SessionRecord> => {
    return invoke<SessionRecord>('workspace_create_session', { path, title })
  }

  deleteSession = async (sessionId: number): Promise<void> => {
    await invoke('workspace_delete_session', { sessionId })
  }

  getActiveSession = async (path: string): Promise<SessionRecord> => {
    return invoke<SessionRecord>('workspace_get_active_session', { path })
  }

  setActiveSession = async (path: string, sessionId: number): Promise<void> => {
    await invoke('workspace_set_active_session', { path, sessionId })
  }

  clearActiveSession = async (path: string): Promise<void> => {
    await invoke('workspace_clear_active_session', { path })
  }

  // ===== Message operations =====

  getMessages = async (sessionId: number, limit?: number, beforeId?: number): Promise<Message[]> => {
    return invoke<Message[]>('workspace_get_messages', { sessionId, limit, beforeId })
  }

  // ===== Recent workspace management =====

  listRecent = async (limit?: number): Promise<WorkspaceRecord[]> => {
    return invoke<WorkspaceRecord[]>('workspace_get_recent', { limit })
  }

  addRecentWorkspace = async (path: string): Promise<void> => {
    await invoke('workspace_add_recent', { path })
  }

  maintainWorkspaces = async (): Promise<[number, number]> => {
    return invoke<[number, number]>('workspace_maintain')
  }

  // ===== Project rules management =====

  getProjectRules = async (): Promise<string | null> => {
    return invoke<string | null>('workspace_get_project_rules')
  }

  setProjectRules = async (rules: string | null): Promise<void> => {
    await invoke<void>('workspace_set_project_rules', { rules })
  }

  listAvailableRulesFiles = async (cwd: string): Promise<string[]> => {
    return invoke<string[]>('workspace_list_rules_files', { cwd })
  }

  // ===== Run Actions =====

  listRunActions = async (path: string): Promise<RunActionRecord[]> => {
    return invoke<RunActionRecord[]>('workspace_list_run_actions', { path })
  }

  createRunAction = async (path: string, name: string, command: string): Promise<RunActionRecord> => {
    return invoke<RunActionRecord>('workspace_create_run_action', { path, name, command })
  }

  updateRunAction = async (id: string, name: string, command: string): Promise<void> => {
    await invoke('workspace_update_run_action', { id, name, command })
  }

  deleteRunAction = async (id: string): Promise<void> => {
    await invoke('workspace_delete_run_action', { id })
  }

  setSelectedRunAction = async (path: string, actionId: string | null): Promise<void> => {
    await invoke('workspace_set_selected_run_action', { path, actionId })
  }

  // ===== Preferences =====

  getPreferences = async (keys: string[]): Promise<Record<string, string>> => {
    return invoke<Record<string, string>>('preferences_get_batch', { keys })
  }

  setPreference = async (key: string, value: string | null): Promise<void> => {
    await invoke('preferences_set', { key, value })
  }
}

export const workspaceApi = new WorkspaceApi()

export default workspaceApi
