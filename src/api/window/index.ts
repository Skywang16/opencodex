/**
 * Window management API
 *
 * Provides unified interface for window management, including:
 * - File drag and drop event listening
 */

import { invoke } from '@/utils/request'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { PlatformInfo, WindowStateSnapshot, WindowStateUpdate } from './types'

/**
 * Window API interface class
 */
export class WindowApi {
  private alwaysOnTopState = false
  private platformInfoCache: PlatformInfo | null = null

  // ===== Window State =====

  getState = async (refresh: boolean = false): Promise<WindowStateSnapshot> => {
    const state = await invoke<WindowStateSnapshot>('window_state_get', { refresh })
    this.alwaysOnTopState = state.alwaysOnTop
    this.platformInfoCache = state.platformInfo
    return state
  }

  updateState = async (update: WindowStateUpdate): Promise<WindowStateSnapshot> => {
    const state = await invoke<WindowStateSnapshot>('window_state_update', { update })
    this.alwaysOnTopState = state.alwaysOnTop
    this.platformInfoCache = state.platformInfo
    return state
  }

  setAlwaysOnTop = async (alwaysOnTop: boolean): Promise<void> => {
    await this.updateState({ alwaysOnTop })
  }

  toggleAlwaysOnTop = async (): Promise<boolean> => {
    const next = !this.alwaysOnTopState
    await this.updateState({ alwaysOnTop: next })
    return next
  }

  getAlwaysOnTopState = (): boolean => {
    return this.alwaysOnTopState
  }

  getPlatformInfo = async (): Promise<PlatformInfo> => {
    if (this.platformInfoCache) {
      return this.platformInfoCache
    }

    const state = await this.getState(false)
    return state.platformInfo
  }

  isMac = async (): Promise<boolean> => {
    const platformInfo = await this.getPlatformInfo()
    return platformInfo.is_mac
  }

  getCurrentDirectory = async (): Promise<string> => {
    const state = await this.getState(false)
    return state.currentDirectory
  }

  getHomeDirectory = async (): Promise<string> => {
    const state = await this.getState(false)
    return state.homeDirectory
  }

  // ===== File handling =====

  handleFileOpen = async (path: string): Promise<string> => {
    return await invoke<string>('file_handle_open', { path })
  }

  // ===== Event listening =====

  /**
   * Listen to startup file event
   */
  onStartupFile = async (callback: (filePath: string) => void): Promise<UnlistenFn> => {
    return await listen<string>('startup-file', event => {
      callback(event.payload)
    })
  }

  /**
   * Listen to file drag and drop event (application icon drag and drop)
   */
  onFileDropped = async (callback: (filePath: string) => void): Promise<UnlistenFn> => {
    return await listen<string>('file-dropped', event => {
      callback(event.payload)
    })
  }

  /**
   * Listen to window drag and drop event
   */
  onWindowDragDrop = async (callback: (filePath: string) => void): Promise<UnlistenFn> => {
    const webview = getCurrentWebview()
    return await webview.onDragDropEvent(event => {
      if (
        event.event === 'tauri://drag-drop' &&
        event.payload &&
        'paths' in event.payload &&
        event.payload.paths &&
        event.payload.paths.length > 0
      ) {
        callback(event.payload.paths[0])
      }
    })
  }
}

export const windowApi = new WindowApi()

export type * from './types'

// Default export
export default windowApi
