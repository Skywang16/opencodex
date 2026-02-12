/**
 * LLM core API
 *
 * Provides a unified interface for large language models, including:
 * - Model invocation
 * - Streaming processing
 * - Model management
 * - Models.dev API integration
 */

import { invoke } from '@/utils/request'
import { llmChannelApi } from '@/api/channel/llm'
import type { ModelsDevProviderInfo, ModelsDevModelInfo } from '@/types/domain/ai'

export interface NativeLLMRequest extends Record<string, unknown> {
  abortSignal?: AbortSignal
}
export type NativeLLMResponse = unknown
export type NativeLLMStreamChunk = { type: string; [key: string]: unknown }

/**
 * LLM API interface class
 */
export class LLMApi {
  /**
   * Regular LLM call
   */
  call = async (request: NativeLLMRequest): Promise<NativeLLMResponse> => {
    return await invoke<NativeLLMResponse>('llm_call', { request })
  }

  /**
   * Streaming LLM call
   */
  callStream = async (request: NativeLLMRequest): Promise<ReadableStream<NativeLLMStreamChunk>> => {
    // Handle abort signal if provided
    if (request.abortSignal) {
      request.abortSignal.addEventListener('abort', () => {
        this.cancelStream().catch(console.warn)
      })
    }

    // Use unified Channel API
    return llmChannelApi.createStream({ request })
  }

  /**
   * Get available model list
   */
  getAvailableModels = async (): Promise<string[]> => {
    return await invoke<string[]>('llm_get_available_models')
  }

  /**
   * Test model connection
   */
  testModelConnection = async (modelId: string): Promise<boolean> => {
    return await invoke<boolean>('llm_test_model_connection', { modelId })
  }

  /**
   * Cancel streaming call
   */
  cancelStream = async (): Promise<void> => {
    return llmChannelApi.cancelStream()
  }

  // ============================================================================
  // Models.dev API
  // ============================================================================

  /**
   * Get providers from models.dev API
   */
  getModelsDevProviders = async (): Promise<ModelsDevProviderInfo[]> => {
    return await invoke<ModelsDevProviderInfo[]>('llm_get_models_dev_providers')
  }

  /**
   * Refresh models from models.dev API
   */
  refreshModelsDev = async (): Promise<void> => {
    return await invoke<void>('llm_refresh_models_dev')
  }

  /**
   * Get model info by provider and model ID
   */
  getModelInfo = async (providerId: string, modelId: string): Promise<ModelsDevModelInfo | null> => {
    return await invoke<ModelsDevModelInfo | null>('llm_get_model_info', {
      providerId,
      modelId,
    })
  }
}

export const llmApi = new LLMApi()

// Default export
export default llmApi
