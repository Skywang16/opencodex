/**
 * Type definitions related to configuration module
 */

import type { ShortcutsConfig } from '@/types/domain/shortcuts'

// Config API error class
export class ConfigApiError extends Error {
  constructor(
    message: string,
    public cause?: unknown
  ) {
    super(message)
    this.name = 'ConfigApiError'
  }
}

export interface AppConfig {
  version: string
  app: {
    language: string
    confirm_on_exit: boolean
    startup_behavior: string
  }
  appearance: {
    ui_scale: number
    animations_enabled: boolean
    theme_config: {
      terminal_theme: string
      light_theme: string
      dark_theme: string
      follow_system: boolean
    }
    font: {
      family: string
      size: number
      weight: string
      style: string
      lineHeight: number
      letterSpacing: number
    }
  }
  terminal: {
    scrollback: number
    shell: {
      default: string
      args: string[]
      working_directory: string
    }
    cursor: {
      style: string
      blink: boolean
      color: string
      thickness: number
    }
    behavior: {
      close_on_exit: boolean
      confirm_close: boolean
    }
  }
  shortcuts: {
    global: ShortcutsConfig
    terminal: ShortcutsConfig
    custom: ShortcutsConfig
  }
}

export interface ConfigFileInfo {
  path: string
  exists: boolean
  lastModified?: number
}

// ===== Configuration section update types =====

export interface ConfigSectionUpdate<T = unknown> {
  section: string
  updates: Partial<T>
}

// ===== Theme-related types =====

export type { Theme, ThemeConfigStatus } from '@/types'
