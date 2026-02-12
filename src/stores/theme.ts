/**
 * Theme state management store
 * Uses Pinia for centralized theme state management
 */

import { themeAPI } from '@/api/config'
import type { Theme, ThemeConfigStatus } from '@/types/domain/theme'
import { applyThemeToUI } from '@/utils/themeApplier'
import { defineStore } from 'pinia'
import { computed, readonly, ref } from 'vue'

enum ThemeOperationState {
  IDLE = 'idle',
  SWITCHING = 'switching',
  UPDATING_CONFIG = 'updating_config',
  ERROR = 'error',
}

interface ThemeOperation {
  type: 'SWITCH_THEME' | 'SET_FOLLOW_SYSTEM' | 'LOAD_CONFIG'
  payload?: {
    themeName?: string
    followSystem?: boolean
    lightTheme?: string
    darkTheme?: string
  }
  timestamp: number
}

interface StateSnapshot {
  configStatus: ThemeConfigStatus | null
  currentTheme: Theme | null
}

export const useThemeStore = defineStore('theme', () => {
  const configStatus = ref<ThemeConfigStatus | null>(null)
  const currentTheme = ref<Theme | null>(null)
  const availableThemes = ref<Theme[]>([])

  const operationState = ref<ThemeOperationState>(ThemeOperationState.IDLE)
  const error = ref<string | null>(null)
  const lastOperation = ref<ThemeOperation | null>(null)

  const themeConfig = computed(() => configStatus.value?.themeConfig || null)
  const currentThemeName = computed(() => configStatus.value?.currentThemeName || '')
  const isSystemDark = computed(() => configStatus.value?.isSystemDark)
  const isFollowingSystem = computed(() => themeConfig.value?.followSystem || false)
  const isLoading = computed(() => operationState.value !== ThemeOperationState.IDLE)

  const themeOptions = computed(() => {
    const currentName = currentThemeName.value
    return availableThemes.value.map(theme => ({
      value: theme.name,
      label: theme.name,
      type: theme.themeType,
      isCurrent: theme.name === currentName,
      // Full theme data for preview
      ui: theme.ui,
    }))
  })

  // Core business logic

  /**
   * Theme switching core logic
   * Uses state machine pattern to manage complex switching flow
   */
  class ThemeSwitcher {
    private store = {
      configStatus,
      currentTheme,
      operationState,
      error,
      lastOperation,
    }

    switchToTheme = async (themeName: string): Promise<void> => {
      const operation: ThemeOperation = {
        type: 'SWITCH_THEME',
        payload: { themeName },
        timestamp: Date.now(),
      }

      return this.executeOperation(operation, async () => {
        // 1. Optimistic update first, immediately update UI state
        this.optimisticUpdateTheme(themeName)

        // 2. Call backend API to switch theme
        await themeAPI.setTerminalTheme(themeName)

        // 3. Get latest actual theme data
        const newTheme = await themeAPI.getCurrentTheme()
        this.store.currentTheme.value = newTheme

        // 4. Apply actual theme to UI (override optimistic update)
        applyThemeToUI(newTheme)
      })
    }

    setFollowSystem = async (followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> => {
      const operation: ThemeOperation = {
        type: 'SET_FOLLOW_SYSTEM',
        payload: { followSystem, lightTheme, darkTheme },
        timestamp: Date.now(),
      }

      return this.executeOperation(operation, async () => {
        // 1. Optimistic update first, immediately update config state
        this.optimisticUpdateFollowSystem(followSystem, lightTheme, darkTheme)

        // 2. Call backend API to sync config
        await themeAPI.setFollowSystemTheme(followSystem, lightTheme, darkTheme)

        // 3. Get current theme data that should be used
        const newTheme = await themeAPI.getCurrentTheme()
        this.store.currentTheme.value = newTheme

        // 4. Apply theme to UI
        applyThemeToUI(newTheme)
      })
    }

    private executeOperation = async (operation: ThemeOperation, action: () => Promise<void>): Promise<void> => {
      // Save current state for rollback
      const snapshot = this.createStateSnapshot()

      try {
        this.store.operationState.value = ThemeOperationState.SWITCHING
        this.store.error.value = null
        this.store.lastOperation.value = operation

        await action()

        this.store.operationState.value = ThemeOperationState.IDLE
      } catch (err) {
        // Rollback state
        this.restoreStateSnapshot(snapshot)
        this.store.operationState.value = ThemeOperationState.ERROR
        this.store.error.value = err instanceof Error ? err.message : String(err)
        throw err
      }
    }

    private optimisticUpdateTheme = (themeName: string): void => {
      if (this.store.configStatus.value) {
        this.store.configStatus.value = {
          ...this.store.configStatus.value,
          currentThemeName: themeName,
          themeConfig: {
            ...this.store.configStatus.value.themeConfig,
            terminalTheme: themeName,
            followSystem: false,
          },
        }
      }
    }

    private optimisticUpdateFollowSystem = (followSystem: boolean, lightTheme?: string, darkTheme?: string): void => {
      if (this.store.configStatus.value) {
        this.store.configStatus.value = {
          ...this.store.configStatus.value,
          themeConfig: {
            ...this.store.configStatus.value.themeConfig,
            followSystem,
            ...(lightTheme && { lightTheme }),
            ...(darkTheme && { darkTheme }),
          },
        }
      }
    }

    private createStateSnapshot = (): StateSnapshot => {
      return {
        configStatus: this.store.configStatus.value,
        currentTheme: this.store.currentTheme.value,
      }
    }

    private restoreStateSnapshot = (snapshot: StateSnapshot): void => {
      this.store.configStatus.value = snapshot.configStatus
      this.store.currentTheme.value = snapshot.currentTheme

      // If there's a current theme, reapply to UI to rollback visual effects
      if (snapshot.currentTheme) {
        applyThemeToUI(snapshot.currentTheme)
      }
    }
  }

  const themeSwitcher = new ThemeSwitcher()

  // Data loading

  const loadThemeConfigStatus = async (): Promise<void> => {
    try {
      operationState.value = ThemeOperationState.UPDATING_CONFIG
      const [status, themes] = await Promise.all([themeAPI.getThemeConfigStatus(), themeAPI.getAvailableThemes()])
      configStatus.value = status
      availableThemes.value = themes
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      operationState.value = ThemeOperationState.IDLE
    }
  }

  const loadCurrentTheme = async (): Promise<void> => {
    try {
      const theme = await themeAPI.getCurrentTheme()
      currentTheme.value = theme

      // Apply theme to UI
      if (theme) {
        applyThemeToUI(theme)
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  // Initialization and cleanup

  const initialize = async (): Promise<void> => {
    await Promise.all([loadThemeConfigStatus(), loadCurrentTheme()])
  }

  const clearError = (): void => {
    error.value = null
  }

  return {
    configStatus: readonly(configStatus),
    currentTheme: readonly(currentTheme),
    availableThemes: readonly(availableThemes),
    operationState: readonly(operationState),
    error: readonly(error),
    lastOperation: readonly(lastOperation),

    themeConfig,
    currentThemeName,
    isSystemDark,
    isFollowingSystem,
    isLoading,
    themeOptions,

    switchToTheme: themeSwitcher.switchToTheme.bind(themeSwitcher),
    setFollowSystem: themeSwitcher.setFollowSystem.bind(themeSwitcher),
    enableFollowSystem: (lightTheme: string, darkTheme: string) =>
      themeSwitcher.setFollowSystem(true, lightTheme, darkTheme),
    disableFollowSystem: () => themeSwitcher.setFollowSystem(false),

    initialize,
    loadThemeConfigStatus,
    loadCurrentTheme,
    clearError,
  }
})
