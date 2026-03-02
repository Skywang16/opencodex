export type { AppConfig, ConfigFileInfo, ConfigSectionUpdate } from '@/types/domain/config'
export type { Theme, ThemeConfigStatus } from '@/types'

export class ConfigApiError extends Error {
  constructor(
    message: string,
    public cause?: unknown
  ) {
    super(message)
    this.name = 'ConfigApiError'
  }
}
