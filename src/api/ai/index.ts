import type { AIModelConfig } from '@/types/domain/ai'
import { AuthType } from '@/types/oauth'
import { invoke } from '@/utils/request'

export class AiApi {
  // ── Model CRUD ────────────────────────────────────────────────────────────

  getModels = async (): Promise<AIModelConfig[]> => {
    return await invoke<AIModelConfig[]>('ai_models_get')
  }

  addModel = async (model: AIModelConfig): Promise<AIModelConfig> => {
    return await invoke<AIModelConfig>('ai_models_add', { model })
  }

  updateModel = async (model: AIModelConfig): Promise<AIModelConfig> => {
    return await invoke<AIModelConfig>('ai_models_update', { model })
  }

  deleteModel = async (modelId: string): Promise<void> => {
    await invoke<void>('ai_models_remove', { modelId })
  }

  testModel = async (model: AIModelConfig): Promise<void> => {
    await invoke<void>('ai_models_test', { model })
  }

  /** Helper: build a new AIModelConfig from form data */
  static buildModel(params: {
    id?: string
    providerId: string
    authType: AuthType
    displayName: string
    model: string
    apiUrl?: string
    apiKey?: string
    oauthRefreshToken?: string
    oauthAccessToken?: string
    oauthExpiresAt?: number
    oauthMetadata?: Record<string, unknown>
    modelType?: AIModelConfig['modelType']
    options?: AIModelConfig['options']
  }): AIModelConfig {
    return {
      id: params.id || crypto.randomUUID(),
      providerId: params.providerId,
      authType: params.authType,
      displayName: params.displayName,
      model: params.model,
      modelType: params.modelType ?? 'chat',
      apiUrl: params.apiUrl,
      apiKey: params.apiKey,
      oauthRefreshToken: params.oauthRefreshToken,
      oauthAccessToken: params.oauthAccessToken,
      oauthExpiresAt: params.oauthExpiresAt,
      oauthMetadata: params.oauthMetadata,
      options: params.options,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    }
  }
}

export const aiApi = new AiApi()
export type * from './types'
export default aiApi
