/**
 * Completion feature-related type definitions
 */

// ===== Completion-Related Types =====

export interface CompletionItem {
  text: string
  displayText?: string
  description?: string
  kind: string
  score: number
  source: string
  icon?: string
  category?: string
  priority?: number
}

export interface CompletionRequest {
  input: string
  cursorPosition: number
  workingDirectory: string
  maxResults?: number
}

export interface CompletionResponse {
  items: CompletionItem[]
  replaceStart: number
  replaceEnd: number
  hasMore: boolean
}

// Uses unified CompletionItem type

// ===== Statistics Types =====

export interface CompletionStats {
  providerCount: number
  cacheStats?: {
    totalEntries: number
    capacity: number
    expiredEntries: number
    hitRate: number
  }
}

// ===== Completion Engine Status Types =====

export interface CompletionEngineStatus {
  initialized: boolean
  ready: boolean
}

// ===== Completion Operation Result Types =====

export interface CompletionResult<T = CompletionResponse> {
  success: boolean
  data?: T
  error?: string
}

// ===== Retry Options Types =====

export interface CompletionRetryOptions {
  retries?: number
  retryDelay?: number
}
