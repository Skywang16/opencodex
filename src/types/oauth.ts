// Authentication types

export enum AuthType {
  ApiKey = 'api_key',
  OAuth = 'oauth',
}

// OAuth Provider types (used only for flow initiation)
export enum OAuthProvider {
  OpenAiCodex = 'openai_codex',
  ClaudePro = 'claude_pro',
  GeminiAdvanced = 'gemini_advanced',
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
}
