/**
 * LLM core API
 *
 * Provides a unified interface for large language models, including:
 * - Model invocation
 * - Streaming processing
 * - Model management
 * - Models.dev API integration
 */

import { llmChannelApi } from '@/api/channel/llm'
import type { ModelsDevProvider, PresetModel, ProviderMetadata } from '@/types/domain/ai'
import { invoke } from '@/utils/request'

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
    const { abortSignal: _, ...serializableRequest } = request
    return await invoke<NativeLLMResponse>('llm_call', { request: serializableRequest })
  }

  /**
   * Streaming LLM call
   */
  callStream = async (request: NativeLLMRequest): Promise<ReadableStream<NativeLLMStreamChunk>> => {
    if (request.abortSignal) {
      request.abortSignal.addEventListener('abort', () => {
        this.cancelStream().catch(console.warn)
      }, { once: true })
    }

    const { abortSignal: _, ...serializableRequest } = request
    return llmChannelApi.createStream({ request: serializableRequest })
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

  /**
   * Get providers (hardcoded registry)
   */
  getProviders = async (): Promise<ProviderMetadata[]> => {
    return await invoke<ProviderMetadata[]>('llm_get_providers')
  }

  /**
   * Get model info by provider and model ID (from hardcoded registry)
   */
  getModelInfo = async (providerId: string, modelId: string): Promise<PresetModel | null> => {
    const providers = await this.getProviders()
    const provider = providers.find(p => p.providerType === providerId)
    return provider?.presetModels.find(m => m.id === modelId) ?? null
  }

  /**
   * Get providers from models.dev (dynamic, up-to-date)
   */
  getModelsDevProviders = async (): Promise<ModelsDevProvider[]> => {
    return await invoke<ModelsDevProvider[]>('llm_get_models_dev_providers')
  }

  /**
   * Force refresh models from models.dev API
   */
  refreshModelsDev = async (): Promise<void> => {
    await invoke<void>('llm_refresh_models_dev')
  }
}

export const llmApi = new LLMApi()

// Default export
export default llmApi
