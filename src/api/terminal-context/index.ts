/**
 * Terminal context management API
 *
 * Provides a unified interface for terminal context management, including:
 * - Active terminal management
 * - Terminal context queries
 * - Context cache management
 */

import { invoke } from '@/utils/request'
import type { TerminalContext } from './types'

/**
 * Terminal context API interface class
 */
export class TerminalContextApi {
  // ===== Active Terminal Management =====

  /**
   * Set active terminal pane ID
   * @param paneId Pane ID
   */
  setActivePaneId = async (paneId: number): Promise<void> => {
    await invoke('terminal_context_set_active_pane', { paneId })
  }

  /**
   * Get current active terminal pane ID
   * @returns Active terminal pane ID, or null if there is no active terminal
   */
  getActivePaneId = async (): Promise<number | null> => {
    return await invoke<number | null>('terminal_context_get_active_pane')
  }

  // ===== Terminal Context Queries =====

  /**
   * Get terminal context information
   * @param paneId Optional pane ID, if not provided, gets the context of the active terminal
   * @returns Terminal context information
   */
  getTerminalContext = async (paneId?: number): Promise<TerminalContext> => {
    return await invoke<TerminalContext>('terminal_context_get', { paneId })
  }

  /**
   * Get active terminal context information
   * @returns Active terminal context information
   */
  getActiveTerminalContext = async (): Promise<TerminalContext> => {
    return await invoke<TerminalContext>('terminal_context_get_active')
  }

  // ===== Convenience Methods =====

  /**
   * Get current working directory of specified terminal
   * @param paneId Optional pane ID, if not provided, gets the CWD of the active terminal
   * @returns Current working directory path
   */
  getCurrentWorkingDirectory = async (paneId?: number): Promise<string | null> => {
    const context = await this.getTerminalContext(paneId)
    return context.currentWorkingDirectory
  }

  /**
   * Get shell type of specified terminal
   * @param paneId Optional pane ID, if not provided, gets the shell type of the active terminal
   * @returns Shell type
   */
  getShellType = async (paneId?: number): Promise<string | null> => {
    const context = await this.getTerminalContext(paneId)
    return context.shellType
  }

  /**
   * Check if shell integration is enabled for specified terminal
   * @param paneId Optional pane ID, if not provided, checks the active terminal
   * @returns Whether shell integration is enabled
   */
  isShellIntegrationEnabled = async (paneId?: number): Promise<boolean> => {
    const context = await this.getTerminalContext(paneId)
    return context.shellIntegrationEnabled
  }

  /**
   * Check if terminal exists and is accessible
   * @param paneId Pane ID
   * @returns Whether terminal exists
   */
  terminalExists = async (paneId: number): Promise<boolean> => {
    await this.getTerminalContext(paneId)
    return true
  }
}

export const terminalContextApi = new TerminalContextApi()
export type * from './types'
export default terminalContextApi
