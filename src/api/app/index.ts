/**
 * Application management API
 *
 * Provides application-level event listening and management functionality
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/**
 * Application API interface class
 */
export class AppApi {
  /**
   * Listen for clear all tabs event (triggered when macOS window closes)
   */
  onClearAllTabs = async (callback: () => void | Promise<void>): Promise<UnlistenFn> => {
    return await listen('clear-all-tabs', async () => {
      await callback()
    })
  }

  /**
   * Listen for custom events
   */
  onCustomEvent = async <T = unknown>(
    eventName: string,
    callback: (payload: T) => void | Promise<void>
  ): Promise<UnlistenFn> => {
    return await listen<T>(eventName, async event => {
      await callback(event.payload)
    })
  }
}

export const appApi = new AppApi()

// Default export
export default appApi
