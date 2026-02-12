// OAuth authentication types
export enum AuthType {
  ApiKey = 'api_key',
  OAuth = 'oauth',
}

// OAuth Provider types
export enum OAuthProvider {
  OpenAiCodex = 'openai_codex',
  ClaudePro = 'claude_pro',
  GeminiAdvanced = 'gemini_advanced',
}

// OAuth configuration
export interface OAuthConfig {
  provider: OAuthProvider
  refreshToken: string
  accessToken?: string
  expiresAt?: number
  metadata?: Record<string, unknown>
}

// OAuth flow information
export interface OAuthFlowInfo {
  flowId: string
  authorizeUrl: string
  provider: string
}

// OAuth status
export enum OAuthStatus {
  NotAuthorized = 'not_authorized',
  Authorized = 'authorized',
  TokenExpired = 'token_expired',
  Authorizing = 'authorizing',
}

// Provider information
export interface ProviderInfo {
  id: OAuthProvider
  name: string
  description: string
  icon: string
  available: boolean
}
