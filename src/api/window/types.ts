/**
 * Type definitions related to window module
 */

export interface PlatformInfo {
  platform: string
  arch: string
  os_version: string
  is_mac: boolean
}

export interface WindowStateSnapshot {
  alwaysOnTop: boolean
  currentDirectory: string
  homeDirectory: string
  platformInfo: PlatformInfo
  timestamp: number
}

export interface WindowStateUpdate {
  alwaysOnTop?: boolean
  refreshDirectories?: boolean
}
