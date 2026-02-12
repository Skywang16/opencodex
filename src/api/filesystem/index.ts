/**
 * Filesystem API
 *
 * Provides unified interface for filesystem operations, including:
 * - File reading
 * - Directory operations
 * - File metadata
 */

import { invoke as appInvoke } from '@/utils/request'
import { invoke } from '@tauri-apps/api/core'

/**
 * Filesystem API interface class
 */
export class FilesystemApi {
  /**
   * Read text file
   */
  readTextFile = async (path: string): Promise<ArrayBuffer> => {
    return await invoke<ArrayBuffer>('plugin:fs|read_text_file', { path })
  }

  /**
   * Check if file or directory exists
   */
  exists = async (path: string): Promise<boolean> => {
    return await invoke<boolean>('plugin:fs|exists', { path })
  }

  /**
   * Get file or directory metadata (using Tauri fs plugin's stat interface)
   */
  getMetadata = async (
    path: string
  ): Promise<{ isDir?: boolean; size?: number; isFile?: boolean; isSymlink?: boolean }> => {
    // tauri-plugin-fs v2 uses 'stat', permission corresponds to "fs:allow-stat" in capabilities
    return await invoke<{ isDir?: boolean; size?: number; isFile?: boolean; isSymlink?: boolean }>('plugin:fs|stat', {
      path,
    })
  }

  /**
   * Check if it is a directory
   */
  isDirectory = async (path: string): Promise<boolean> => {
    const metadata = await this.getMetadata(path)
    return metadata.isDir || false
  }

  /**
   * Get file size
   */
  getFileSize = async (path: string): Promise<number> => {
    const metadata = await this.getMetadata(path)
    return metadata.size || 0
  }

  /**
   * Read directory contents (including gitignore status)
   */
  readDir = async (
    path: string
  ): Promise<
    Array<{
      name: string
      isDirectory: boolean
      isFile: boolean
      isSymlink: boolean
      isIgnored: boolean
    }>
  > => {
    return await appInvoke('fs_read_dir', { path })
  }

  /**
   * List directory (backend command, full .gitignore semantics, recursive optional)
   */
}

export const filesystemApi = new FilesystemApi()

// Default export
export default filesystemApi
