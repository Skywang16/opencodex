/**
 * Storage management API
 *
 * Responsibility boundary: only handles Runtime state.
 * - Runtime: backend Mux runtime terminal state (e.g., real-time CWD)
 *
 * Config(TOML) should use `src/api/config` (to avoid duplicate write entry points).
 * UI layout persistence uses `src/api/workspace` (preferences_get_batch / preferences_set).
 */

import { invoke } from '@/utils/request'
import type { RuntimeTerminalState } from './types'

/**
 * Storage API interface class
 */
export class StorageApi {
  // ===== Terminal state management =====

  /** Get runtime state of all terminals */
  getTerminalsState = async (): Promise<RuntimeTerminalState[]> => {
    return await invoke<RuntimeTerminalState[]>('storage_get_terminals_state')
  }

  /** Get runtime state of a single terminal */
  getTerminalState = async (paneId: number): Promise<RuntimeTerminalState | null> => {
    return await invoke<RuntimeTerminalState | null>('storage_get_terminal_state', { paneId })
  }

  /** Get current working directory of specified terminal */
  getTerminalCwd = async (paneId: number): Promise<string> => {
    return await invoke<string>('storage_get_terminal_cwd', { paneId })
  }
}

export const storageApi = new StorageApi()
export type * from './types'
export default storageApi
