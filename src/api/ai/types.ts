import type { AIModelConfig } from '@/types'
import type { AuthType, OAuthConfig } from '@/types/oauth'

export type { AIModelConfig, AIModelsConfig, AISettings } from '@/types'

export interface AIModelCreateInput {
  provider: AIModelConfig['provider']
  authType: AuthType
  apiUrl?: string
  apiKey?: string
  model: string
  displayName?: string
  modelType: AIModelConfig['modelType']
  options?: AIModelConfig['options']
  oauthConfig?: OAuthConfig
}

export type AIModelUpdateChanges = Partial<
  Omit<
    Pick<
      AIModelConfig,
      'provider' | 'authType' | 'apiUrl' | 'apiKey' | 'model' | 'displayName' | 'modelType' | 'options' | 'oauthConfig'
    >,
    'oauthConfig'
  >
> & {
  oauthConfig?: OAuthConfig | null
}

export interface AIModelUpdateInput {
  id: string
  changes: AIModelUpdateChanges
}

export interface AIModelTestConnectionInput extends Omit<AIModelCreateInput, 'oauthConfig'> {}
