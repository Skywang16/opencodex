/**
 * Shell Integration API
 *
 * Provides unified interface for Shell Integration, including:
 * - Shell Integration setup and status checking
 * - Working directory management
 * - OSC sequence processing
 */

import { invoke } from '@/utils/request'

/**
 * Shell Integration API interface class
 */
export class ShellIntegrationApi {
  // ===== Shell Integration setup =====

  /**
   * Setup Shell Integration
   * @param paneId Terminal pane ID
   * @param silent Whether to setup silently
   */
  setupShellIntegration = async (paneId: number, silent: boolean = true): Promise<void> => {
    await invoke('shell_pane_setup_integration', { paneId, silent })
  }

  /**
   * Check Shell Integration status
   * @param paneId Terminal pane ID
   */
  checkShellIntegrationStatus = async (paneId: number): Promise<boolean> => {
    const state = await this.getPaneShellState<{ integration_enabled?: boolean } | null>(paneId)
    return !!state?.integration_enabled
  }

  /**
   * Get Shell state snapshot of pane (including node_version, etc.)
   */
  getPaneShellState = async <T = { node_version?: string | null } | null>(paneId: number): Promise<T> => {
    // Directly call backend command shell_pane_get_state
    // Returns FrontendPaneState, can destructure node_version field as needed
    return await invoke<T>('shell_pane_get_state', { paneId })
  }
}

export const shellIntegrationApi = new ShellIntegrationApi()

// Default export
export default shellIntegrationApi
