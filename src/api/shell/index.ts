/**
 * Shell management API
 *
 * Provides unified interface for Shell management, including:
 * - Shell discovery and validation
 * - Configuration management
 * - Feature detection
 */

import { invoke } from '@/utils/request'
import type { BackgroundCommandResult, ShellInfo } from './types'

/**
 * Shell API interface class
 */
export class ShellApi {
  // ===== Basic operations =====

  getAvailableShells = async (): Promise<ShellInfo[]> => {
    return await invoke<ShellInfo[]>('terminal_get_available_shells')
  }

  getDefaultShell = async (): Promise<ShellInfo> => {
    return await invoke<ShellInfo>('terminal_get_default_shell')
  }

  validateShellPath = async (path: string): Promise<boolean> => {
    return await invoke<boolean>('terminal_validate_shell_path', { path })
  }

  // ===== Search functionality =====

  findShellByName = async (name: string): Promise<ShellInfo | null> => {
    const shells = await this.getAvailableShells()
    return shells.find(shell => shell.name.toLowerCase() === name.toLowerCase()) || null
  }

  findShellByPath = async (path: string): Promise<ShellInfo | null> => {
    const shells = await this.getAvailableShells()
    return shells.find(shell => shell.path === path) || null
  }

  // ===== Background command execution functionality =====

  executeBackgroundProgram = async (
    program: string,
    args: string[],
    workingDirectory?: string
  ): Promise<BackgroundCommandResult> => {
    return await invoke<BackgroundCommandResult>('shell_execute_background_program', {
      program,
      args,
      working_directory: workingDirectory,
    })
  }
}

export const shellApi = new ShellApi()
export type * from './types'
export default shellApi
