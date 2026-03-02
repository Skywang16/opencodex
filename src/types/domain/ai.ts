import type { AuthType } from '../oauth'

export type AIProvider = string

// ============================================================================
// Provider registry types (backend hardcoded providers)
// ============================================================================

export interface ModelCapabilities {
  reasoning: boolean
  toolCall: boolean
  attachment: boolean
}

export interface PresetModel {
  id: string
  name: string
  maxTokens?: number
  contextWindow: number
  description?: string
  capabilities: ModelCapabilities
}

export interface ProviderMetadata {
  providerType: string
  displayName: string
  defaultApiUrl: string
  presetModels: PresetModel[]
}

// ============================================================================
// Models.dev dynamic provider types
// ============================================================================

export interface ModelsDevModel {
  id: string
  name: string
  reasoning: boolean
  toolCall: boolean
  attachment: boolean
  contextWindow: number
  maxOutput: number
}

export interface ModelsDevProvider {
  id: string
  name: string
  apiUrl?: string
  envVars: string[]
  models: ModelsDevModel[]
}

export type ModelType = 'chat' | 'embedding'

// ============================================================================
// AI model config — unified single-row design (auth + model + metadata)
// ============================================================================

export interface AIModelConfig {
  id: string
  // ── provider & auth ─────────────────────────────────────────────
  providerId: string
  authType: AuthType
  displayName: string
  apiUrl?: string
  apiKey?: string
  oauthRefreshToken?: string
  oauthAccessToken?: string
  oauthExpiresAt?: number
  oauthMetadata?: Record<string, unknown>
  // ── model selection ─────────────────────────────────────────────
  model: string
  modelType: ModelType
  options?: {
    maxContextTokens?: number
    temperature?: number
    timeoutSeconds?: number
    dimension?: number
    contextWindow?: number
    maxTokens?: number
    enableDeepThinking?: boolean
    reasoningEffort?: string
  }
  createdAt?: string
  updatedAt?: string
}

// ============================================================================
// OAuth token result — returned by OAuth flow, merged into AIModelConfig
// ============================================================================

export interface OAuthTokenResult {
  providerId: string
  apiUrl?: string
  oauthRefreshToken: string
  oauthAccessToken: string
  oauthExpiresAt?: number
  oauthMetadata?: Record<string, unknown>
}
