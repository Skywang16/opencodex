/**
 * AI-related constant configuration
 */

// ===== Cache Configuration =====
export const AI_CACHE_CONFIG = {
  /** Cache duration (milliseconds) */
  DURATION: 5 * 60 * 1000, // 5 minutes
  /** Model list cache duration */
  MODELS_DURATION: 5 * 60 * 1000,
} as const

// ===== Session Configuration =====
export const AI_SESSION_CONFIG = {
  /** Maximum number of saved sessions */
  MAX_SESSIONS: 50,
  /** Maximum session title length */
  TITLE_MAX_LENGTH: 30,
  /** Storage key name */
  STORAGE_KEY: 'ai-chat-sessions',
} as const

// ===== Streaming Configuration =====
export const AI_STREAMING_CONFIG = {
  /** Default timeout */
  DEFAULT_TIMEOUT: 300000,
  /** Maximum retry count */
  MAX_RETRIES: 3,
  /** Retry delay */
  RETRY_DELAY: 1000,
} as const

// ===== Message Configuration =====
export const AI_MESSAGE_CONFIG = {
  /** Code block ID prefix */
  CODE_BLOCK_ID_PREFIX: 'code',
  /** Copy button text configuration */
  COPY_BUTTON_TEXT: {
    IDLE: 'Copy',
    COPYING: 'Copying...',
    SUCCESS: 'Copied',
    ERROR: 'Copy failed',
  },
  /** Copy status reset delay */
  COPY_RESET_DELAY: 2000,
} as const
