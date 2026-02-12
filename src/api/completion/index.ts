/**
 * Completion management API
 *
 * Provides unified interface for intelligent completion, including:
 * - Completion engine management
 * - Completion suggestion retrieval
 * - Statistics and status monitoring
 */

import { invoke } from '@/utils/request'
import type { CompletionRequest, CompletionResponse, CompletionStats } from './types'

/**
 * Completion API interface class
 */
export class CompletionApi {
  initEngine = async (): Promise<void> => {
    await invoke<void>('completion_init_engine')
  }

  getCompletions = async (request: CompletionRequest): Promise<CompletionResponse> => {
    return await invoke<CompletionResponse>('completion_get', {
      input: request.input,
      cursorPosition: request.cursorPosition,
      workingDirectory: request.workingDirectory,
      maxResults: request.maxResults,
    })
  }

  clearCache = async (): Promise<void> => {
    await invoke<void>('completion_clear_cache')
  }

  getStats = async (): Promise<CompletionStats> => {
    return await invoke<CompletionStats>('completion_get_stats')
  }
}

export const completionApi = new CompletionApi()
export type * from './types'
export default completionApi
